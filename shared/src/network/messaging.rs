// Std.
use std::io::prelude::*;
use std::net::*;
use std::thread;
use std::time::Duration;

// Custom.
use super::net_params::*;
use crate::misc::error::*;

// External.
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes256;
use cmac::{Cmac, Mac};
use num_bigint::{BigUint, RandomBits};
use rand::{Rng, RngCore};
use serde::Serialize;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

// ---------- if changed one change other --------------
type MessageLenType = u32; // maximum message size is 4 GB, but the server has its own limit
const MAX_MESSAGE_LEN: usize = std::u32::MAX as usize;
// ---------------------------------------------------------

/// If total message size exceeds this value it will be split into smaller
/// chunks and send in chunks.
const MAX_MESSAGE_SIZE_UNTIL_SPLITING_IN_BYTES: usize = 8192;

enum IoResult {
    Ok(usize),
    Fin,
    Err(AppError),
    Timeout,
}

/// Initiates secure connection establishment with remote
/// (other side should call `accept_secure_connection_establishment`).
///
/// Generates a secret key that will be used to encrypt network messages.
///
/// Returns `Ok(Vec<u8>)` with the secret key if no errors occurred.
pub fn start_establishing_secure_connection(socket: &mut TcpStream) -> Result<Vec<u8>, AppError> {
    // taken from https://www.rfc-editor.org/rfc/rfc5114#section-2.1
    let p = BigUint::parse_bytes(
            b"B10B8F96A080E01DDE92DE5EAE5D54EC52C99FBCFB06A3C69A6A9DCA52D23B616073E28675A23D189838EF1E2EE652C013ECB4AEA906112324975C3CD49B83BFACCBDD7D90C4BD7098488E9C219A73724EFFD6FAE5644738FAA31A4FF55BCCC0A151AF5F0DC8B4BD45BF37DF365C1A65E68CFDA76D4DA708DF1FB2BC2E4A4371",
            16
        ).unwrap();
    let g = BigUint::parse_bytes(
            b"A4D1CBD5C3FD34126765A442EFB99905F8104DD258AC507FD6406CFF14266D31266FEA1E5C41564B777E690F5504F213160217B4B01B886A5E91547F9E2749F4D7FBD7D3B9A92EE1909D0D2263F80A76A6A24C087A091F531DBF0A0169B6A28AD662A4D18E73AFA32D779D5918D08BC8858F4DCEF97C2A24855E6EEB22B3B2E5",
            16
        ).unwrap();

    // Send 2 values: p (BigUint), g (BigUint) values.
    let p_buf = bincode::serialize(&p);
    let g_buf = bincode::serialize(&g);

    if let Err(e) = p_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let mut p_buf = p_buf.unwrap();

    if let Err(e) = g_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let mut g_buf = g_buf.unwrap();

    let p_len = p_buf.len() as u64;
    let mut p_len = bincode::serialize(&p_len).unwrap();

    let g_len = g_buf.len() as u64;
    let mut g_len = bincode::serialize(&g_len).unwrap();

    let mut pg_send_buf = Vec::new();
    pg_send_buf.append(&mut p_len);
    pg_send_buf.append(&mut p_buf);
    pg_send_buf.append(&mut g_len);
    pg_send_buf.append(&mut g_buf);

    // Send p and g values.
    match write_to_socket(socket, &mut pg_send_buf) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("write timeout"));
        }
    }

    // Generate secret key 'a'.
    let mut rng = rand::thread_rng();
    let a: BigUint = rng.sample(RandomBits::new(A_B_BITS));

    // Generate open key 'A'.
    let a_open = g.modpow(&a, &p);

    // Prepare to send open key 'A'.
    let a_open_buf = bincode::serialize(&a_open);
    if let Err(e) = a_open_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let mut a_open_buf = a_open_buf.unwrap();

    // Send open key 'A'.
    let a_open_len = a_open_buf.len() as u64;
    let a_open_len_buf = bincode::serialize(&a_open_len);
    if let Err(e) = a_open_len_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let mut a_open_len_buf = a_open_len_buf.unwrap();
    a_open_len_buf.append(&mut a_open_buf);
    match write_to_socket(socket, &mut a_open_len_buf) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("write timeout"));
        }
    }

    // Receive open key 'B' size.
    let mut b_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    match read_from_socket_fill_buf(socket, &mut b_open_len_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }

    // Receive open key 'B'.
    let b_open_len = bincode::deserialize::<u64>(&b_open_len_buf);
    if let Err(e) = b_open_len {
        return Err(AppError::new(&e.to_string()));
    }
    let b_open_len = b_open_len.unwrap();
    let mut b_open_buf = vec![0u8; b_open_len as usize];

    match read_from_socket_fill_buf(socket, &mut b_open_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }

    let b_open_big = bincode::deserialize::<BigUint>(&b_open_buf);
    if let Err(e) = b_open_big {
        return Err(AppError::new(&e.to_string()));
    }
    let b_open_big = b_open_big.unwrap();

    // Calculate the secret key.
    let secret_key = b_open_big.modpow(&a, &p);
    let mut secret_key_str = secret_key.to_str_radix(10);

    if secret_key_str.len() < SECRET_KEY_SIZE {
        if secret_key_str.is_empty() {
            return Err(AppError::new("generated secret key is empty"));
        }

        loop {
            secret_key_str += &secret_key_str.clone();

            if secret_key_str.len() >= SECRET_KEY_SIZE {
                break;
            }
        }
    }

    Ok(Vec::from(&secret_key_str[0..SECRET_KEY_SIZE]))
}

/// Accepts secure connection establishment request from remote
/// (other side called or will call `start_establishing_secure_connection`).
///
/// Generates a secret key that will be used to encrypt network messages.
///
/// Returns `Ok(Vec<u8>)` with the secret key if no errors occurred.
pub fn accept_secure_connection_establishment(socket: &mut TcpStream) -> Result<Vec<u8>, AppError> {
    // Generate secret key 'b'.
    let mut rng = rand::thread_rng();
    let b: BigUint = rng.sample(RandomBits::new(A_B_BITS));

    // Receive 2 values: p (BigUint), g (BigUint) values.
    // Get 'p' len.
    let mut p_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    match read_from_socket_fill_buf(socket, &mut p_len_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }
    let p_len = bincode::deserialize::<u64>(&p_len_buf);
    if let Err(e) = p_len {
        return Err(AppError::new(&e.to_string()));
    }
    let p_len = p_len.unwrap();

    // Get 'p' value.
    let mut p_buf = vec![0u8; p_len as usize];
    match read_from_socket_fill_buf(socket, &mut p_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }
    let p_buf = bincode::deserialize::<BigUint>(&p_buf);
    if let Err(e) = p_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let p = p_buf.unwrap();

    // Get 'g' len.
    let mut g_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    match read_from_socket_fill_buf(socket, &mut g_len_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }
    let g_len = bincode::deserialize::<u64>(&g_len_buf);
    if let Err(e) = g_len {
        return Err(AppError::new(&e.to_string()));
    }
    let g_len = g_len.unwrap();

    // Get 'g' value.
    let mut g_buf = vec![0u8; g_len as usize];
    match read_from_socket_fill_buf(socket, &mut g_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }
    let g_buf = bincode::deserialize::<BigUint>(&g_buf);
    if let Err(e) = g_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let g = g_buf.unwrap();

    // Calculate the open key B.
    let b_open = g.modpow(&b, &p);

    // Receive the open key A size.
    let mut a_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    match read_from_socket_fill_buf(socket, &mut a_open_len_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }

    let a_open_len = bincode::deserialize::<u64>(&a_open_len_buf);
    if let Err(e) = a_open_len {
        return Err(AppError::new(&e.to_string()));
    }
    let a_open_len = a_open_len.unwrap();

    // Receive the open key A.
    let mut a_open_buf = vec![0u8; a_open_len as usize];
    match read_from_socket_fill_buf(socket, &mut a_open_buf, None) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("read timeout"));
        }
    }

    let a_open_big = bincode::deserialize::<BigUint>(&a_open_buf);
    if let Err(e) = a_open_big {
        return Err(AppError::new(&e.to_string()));
    }
    let a_open_big = a_open_big.unwrap();

    // Prepare to send open key B.
    let mut b_open_buf = bincode::serialize(&b_open).unwrap();

    // Send open key 'B'.
    let b_open_len = b_open_buf.len() as u64;
    let b_open_len_buf = bincode::serialize(&b_open_len);
    if let Err(e) = b_open_len_buf {
        return Err(AppError::new(&e.to_string()));
    }
    let mut b_open_len_buf = b_open_len_buf.unwrap();
    b_open_len_buf.append(&mut b_open_buf);
    match write_to_socket(socket, &mut b_open_len_buf) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received"));
        }
        IoResult::Err(app_error) => {
            return Err(app_error);
        }
        IoResult::Ok(_) => {}
        IoResult::Timeout => {
            return Err(AppError::new("write timeout"));
        }
    }

    // Calculate the secret key.
    let secret_key = a_open_big.modpow(&b, &p);
    let mut secret_key_str = secret_key.to_str_radix(10);

    if secret_key_str.len() < SECRET_KEY_SIZE {
        if secret_key_str.is_empty() {
            return Err(AppError::new("generated secret key is empty"));
        }

        loop {
            secret_key_str += &secret_key_str.clone();

            if secret_key_str.len() >= SECRET_KEY_SIZE {
                break;
            }
        }
    }

    Ok(Vec::from(&secret_key_str[0..SECRET_KEY_SIZE]))
}

/// Encrypts and sends a message.
///
/// Parameters:
/// - `socket`: socket to use.
/// - `secret_key`: secret key that will be used to encrypt the message.
/// - `message`: message to send.
///
/// Returns `None` if successful, `Some` otherwise.
pub fn send_message<T>(
    socket: &mut TcpStream,
    secret_key: &[u8; SECRET_KEY_SIZE],
    message: T,
) -> Option<AppError>
where
    T: Serialize,
{
    if secret_key.is_empty() {
        return Some(AppError::new(
            "can't send message - secure connected is not established yet",
        ));
    }

    // Get socket remote address.
    let peer_addr = socket.peer_addr();
    if let Err(e) = peer_addr {
        return Some(AppError::new(&format!(
            "failed to get socket peer address (error: {})",
            e
        )));
    }
    let socket_addr = peer_addr.unwrap();

    // Serialize.
    let mut binary_message = bincode::serialize(&message).unwrap();

    // CMAC.
    let mut mac = Cmac::<Aes256>::new_from_slice(secret_key).unwrap();
    mac.update(&binary_message);
    let result = mac.finalize();
    let mut tag_bytes = result.into_bytes().to_vec();
    if tag_bytes.len() != CMAC_TAG_LENGTH {
        return Some(AppError::new(&format!(
            "unexpected tag length: {} != {}",
            tag_bytes.len(),
            CMAC_TAG_LENGTH
        )));
    }

    binary_message.append(&mut tag_bytes);

    // Encrypt message.
    let mut rng = rand::thread_rng();
    let mut iv = [0u8; IV_LENGTH];
    rng.fill_bytes(&mut iv);
    let mut encrypted_binary_message = Aes256CbcEnc::new(secret_key.into(), &iv.into())
        .encrypt_padded_vec_mut::<Pkcs7>(&binary_message);

    // Prepare encrypted message len buffer.
    if encrypted_binary_message.len() + IV_LENGTH > MAX_MESSAGE_LEN {
        // should never happen
        return Some(AppError::new(&format!(
            "resulting message is too big ({} > {})",
            encrypted_binary_message.len() + IV_LENGTH,
            MAX_MESSAGE_LEN
        )));
    }
    let encrypted_len = (encrypted_binary_message.len() + iv.len()) as MessageLenType;
    let encrypted_len_buf = bincode::serialize(&encrypted_len);
    if let Err(e) = encrypted_len_buf {
        return Some(AppError::new(&format!("{:?}", e)));
    }

    // Merge all into one buffer.
    let mut send_buffer = encrypted_len_buf.unwrap();
    send_buffer.append(&mut Vec::from(iv));
    send_buffer.append(&mut encrypted_binary_message);

    if send_buffer.len() <= MAX_MESSAGE_SIZE_UNTIL_SPLITING_IN_BYTES {
        // Send.
        match write_to_socket(socket, &mut send_buffer) {
            IoResult::Fin => {
                return Some(AppError::new("unexpected FIN received"));
            }
            IoResult::Err(app_error) => return Some(app_error),
            IoResult::Ok(_) => {}
            IoResult::Timeout => {
                return Some(AppError::new(&format!(
                    "write timeout (socket: {})",
                    socket_addr
                )));
            }
        }
    } else {
        // Need to split the message in smaller chunks.
        let mut chunks: Vec<&mut [u8]> = send_buffer
            .chunks_mut(MAX_MESSAGE_SIZE_UNTIL_SPLITING_IN_BYTES)
            .collect();
        for chunk in chunks.iter_mut() {
            // Send.
            match write_to_socket(socket, chunk) {
                IoResult::Fin => {
                    return Some(AppError::new("unexpected FIN received"));
                }
                IoResult::Err(app_error) => return Some(app_error),
                IoResult::Ok(_) => {}
                IoResult::Timeout => {
                    return Some(AppError::new(&format!(
                        "write timeout (socket: {})",
                        socket_addr
                    )));
                }
            }
        }
    }

    None
}

/// Waits for next message to arrive.
///
/// ## Arguments
/// - `socket`: socket to use.
/// - `secret_key`: secret key to decrypt message.
/// - `timeout_in_ms`: if specified the operation will have a custom timeout,
/// if not, default timeout of `MAX_WAIT_TIME_IN_READ_WRITE_MS` will be used.
/// - `max_allowed_message_size_in_bytes`: maximum size of allowed message, if
/// the incoming message is bigger than this value an error will be returned.
/// - `is_fin`: will be `true` if the remote socket closed connection.
///
/// ## Return
/// Empty `Ok` array if received FIN from remote connection (connection is being closed).
/// If custom timeout was specified and reached a timeout will return `Ok` with zero length vector,
/// otherwise if custom timeout was not specified will return `AppError`.
pub fn receive_message(
    socket: &mut TcpStream,
    secret_key: &[u8; SECRET_KEY_SIZE],
    timeout_in_ms: Option<u64>,
    max_allowed_message_size_in_bytes: usize,
    is_fin: &mut bool,
) -> Result<Vec<u8>, AppError> {
    if secret_key.is_empty() {
        return Err(AppError::new(
            "can't receive message - secure connected is not established",
        ));
    }

    // Get socket remote address.
    let peer_addr = socket.peer_addr();
    if let Err(e) = peer_addr {
        return Err(AppError::new(&format!(
            "failed to get socket peer address (error: {})",
            e
        )));
    }
    let socket_addr = peer_addr.unwrap();

    // Read total size of an incoming message.
    let mut message_size_buf = vec![0u8; std::mem::size_of::<MessageLenType>()];
    let mut _next_message_size: usize = 0;

    let result = read_from_socket_fill_buf(socket, &mut message_size_buf, timeout_in_ms);
    match result {
        IoResult::Fin => {
            *is_fin = true;
            return Ok(Vec::new());
        }
        IoResult::Err(app_error) => return Err(app_error),
        IoResult::Ok(byte_count) => {
            if byte_count != message_size_buf.len() {
                return Err(AppError::new(&format!(
                    "not all data received (got: {}, expected: {}) (socket: {})",
                    byte_count,
                    message_size_buf.len(),
                    socket_addr
                )));
            }

            let res = bincode::deserialize::<MessageLenType>(&message_size_buf);
            if let Err(e) = res {
                return Err(AppError::new(&format!("{:?} (socket: {})", e, socket_addr)));
            }

            _next_message_size = res.unwrap() as usize;
        }
        IoResult::Timeout => {
            if timeout_in_ms.is_some() {
                return Ok(Vec::new());
            } else {
                return Err(AppError::new(&format!(
                    "read timeout (socket: {})",
                    socket_addr
                )));
            }
        }
    }

    // Check message size.
    if _next_message_size > max_allowed_message_size_in_bytes {
        return Err(AppError::new(&format!(
            "incoming message is too big to receive ({} > {} bytes) (socket: {})",
            _next_message_size, max_allowed_message_size_in_bytes, socket_addr
        )));
    }

    // Receive encrypted message.
    let mut encrypted_message: Vec<u8> = Vec::new();
    if _next_message_size + std::mem::size_of::<MessageLenType>()
        <= MAX_MESSAGE_SIZE_UNTIL_SPLITING_IN_BYTES
    {
        encrypted_message = vec![0u8; _next_message_size as usize];
        match read_from_socket_fill_buf(socket, &mut encrypted_message, None) {
            IoResult::Fin => {
                *is_fin = true;
                return Err(AppError::new(&format!(
                    "unexpected FIN received (socket: {})",
                    socket_addr
                )));
            }
            IoResult::Err(app_error) => return Err(app_error),
            IoResult::Ok(_) => {}
            IoResult::Timeout => {
                return Err(AppError::new(&format!(
                    "read timeout (socket: {})",
                    socket_addr
                )));
            }
        };
    } else {
        // The message is split into multiple chunks.
        let mut bytes_left_to_receive = _next_message_size;

        while bytes_left_to_receive != 0 {
            let mut _chunk: Vec<u8> = Vec::new();

            if bytes_left_to_receive > MAX_MESSAGE_SIZE_UNTIL_SPLITING_IN_BYTES {
                _chunk = vec![0u8; MAX_MESSAGE_SIZE_UNTIL_SPLITING_IN_BYTES];
            } else {
                _chunk = vec![0u8; bytes_left_to_receive];
            }

            match read_from_socket_fill_buf(socket, &mut _chunk, None) {
                IoResult::Fin => {
                    *is_fin = true;
                    return Err(AppError::new(&format!(
                        "unexpected FIN received (socket: {})",
                        socket_addr
                    )));
                }
                IoResult::Err(app_error) => return Err(app_error),
                IoResult::Ok(_) => {}
                IoResult::Timeout => {
                    return Err(AppError::new(&format!(
                        "read timeout (socket: {})",
                        socket_addr
                    )));
                }
            };

            bytes_left_to_receive -= _chunk.len();
            encrypted_message.append(&mut _chunk);
        }
    }

    // Get IV.
    if encrypted_message.len() < IV_LENGTH {
        return Err(AppError::new(&format!(
            "unexpected message length ({}) (socket: {})",
            encrypted_message.len(),
            socket_addr
        )));
    }
    let iv = encrypted_message[..IV_LENGTH].to_vec();
    encrypted_message = encrypted_message[IV_LENGTH..].to_vec();

    // Convert IV.
    let iv = iv.try_into();
    if iv.is_err() {
        return Err(AppError::new("failed to convert iv to generic array"));
    }
    let iv: [u8; IV_LENGTH] = iv.unwrap();

    // Decrypt message.
    let decrypted_message = Aes256CbcDec::new(secret_key.into(), &iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(&encrypted_message);
    if let Err(e) = decrypted_message {
        return Err(AppError::new(&format!("{:?} (socket: {})", e, socket_addr)));
    }
    let mut decrypted_message = decrypted_message.unwrap();

    // CMAC
    let mut mac = Cmac::<Aes256>::new_from_slice(secret_key).unwrap();
    let tag: Vec<u8> = decrypted_message
        .drain(decrypted_message.len().saturating_sub(CMAC_TAG_LENGTH)..)
        .collect();
    mac.update(&decrypted_message);

    // Convert tag.
    let tag = tag.try_into();
    if tag.is_err() {
        return Err(AppError::new("failed to convert cmac tag to generic array"));
    }
    let tag: [u8; CMAC_TAG_LENGTH] = tag.unwrap();

    // Check that tag is correct.
    if let Err(e) = mac.verify(&tag.into()) {
        return Err(AppError::new(&format!("{:?} (socket: {})", e, socket_addr)));
    }

    Ok(decrypted_message)
}

/// Writes the specified buffer to the socket with a timeout.
///
/// The timeout is specified by `MAX_WAIT_TIME_IN_READ_WRITE_MS` constant.
///
/// ## Arguments:
/// - `socket`: socket to write this data to.
/// - `buf`: buffer to write to the socket.
fn write_to_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
    if buf.is_empty() {
        return IoResult::Err(AppError::new("the specified buffer has zero length"));
    }

    let mut total_wait_time_ms: u64 = 0;
    let mut filled_buf_count: usize = 0;
    let initial_buf_len = buf.len();
    let mut temp_buf: Vec<u8> = Vec::from(buf);

    loop {
        if total_wait_time_ms >= MAX_WAIT_TIME_IN_READ_WRITE_MS {
            return IoResult::Timeout;
        }

        match socket.write(&temp_buf) {
            Ok(0) => {
                return IoResult::Fin;
            }
            Ok(n) => {
                filled_buf_count += n;
                total_wait_time_ms = 0;

                if filled_buf_count != initial_buf_len {
                    temp_buf = Vec::from(&temp_buf[filled_buf_count..]);
                    continue;
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                continue;
            }
            Err(e) => {
                return IoResult::Err(AppError::new(&format!(
                    "{:?} (socket {})",
                    e,
                    match socket.peer_addr() {
                        Ok(addr) => {
                            addr.to_string()
                        }
                        Err(_) => {
                            String::new()
                        }
                    }
                )));
            }
        };
    }
}

/// Reads data from the specified socket until the specified buffer is filled with a timeout.
/// Blocks the current thread until the buffer is filled or a timeout is reached.
///
/// ## Arguments
/// - `socket`: socket to read the data from.
/// - `buf`: buffer to write read data.
/// - `timeout_in_ms`: if specified the operation will have a custom timeout,
/// if not, default timeout of `MAX_WAIT_TIME_IN_READ_WRITE_MS` will be used.
fn read_from_socket_fill_buf(
    socket: &mut TcpStream,
    buf: &mut [u8],
    timeout_in_ms: Option<u64>,
) -> IoResult {
    if buf.is_empty() {
        return IoResult::Err(AppError::new("the specified buffer has zero length"));
    }

    let mut total_wait_time_ms: u64 = 0;
    let mut filled_buf_count: usize = 0;
    let mut temp_buf = vec![0u8; buf.len()];

    loop {
        if let Some(timeout_in_ms) = timeout_in_ms {
            if total_wait_time_ms >= timeout_in_ms {
                return IoResult::Timeout;
            }
        } else if total_wait_time_ms >= MAX_WAIT_TIME_IN_READ_WRITE_MS {
            return IoResult::Timeout;
        }

        match socket.read(&mut temp_buf) {
            Ok(0) => {
                return IoResult::Fin;
            }
            Ok(n) => {
                // Write received data to buf.
                buf[filled_buf_count..(n + filled_buf_count)].copy_from_slice(&temp_buf[..n]);

                filled_buf_count += n;
                total_wait_time_ms = 0;

                if filled_buf_count != buf.len() {
                    temp_buf = vec![0u8; buf.len() - filled_buf_count];
                    continue;
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                continue;
            }
            Err(e) => {
                return IoResult::Err(AppError::new(&format!(
                    "{} (socket {})",
                    e,
                    match socket.peer_addr() {
                        Ok(addr) => {
                            addr.to_string()
                        }
                        Err(_) => {
                            String::new()
                        }
                    },
                )));
            }
        };
    }
}
