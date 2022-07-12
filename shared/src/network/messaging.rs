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

enum IoResult {
    Ok(usize),
    Fin,
    Err(AppError),
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
        return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
    }
    let mut p_buf = p_buf.unwrap();

    if let Err(e) = g_buf {
        return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
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
    match write_to_socket(socket, &mut pg_send_buf, true) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received", file!(), line!()));
        }
        IoResult::Err(err) => {
            return Err(err.add_entry(file!(), line!()));
        }
        IoResult::Ok(_) => {}
    }

    // Generate secret key 'a'.
    let mut rng = rand::thread_rng();
    let a: BigUint = rng.sample(RandomBits::new(A_B_BITS));

    // Generate open key 'A'.
    let a_open = g.modpow(&a, &p);

    // Prepare to send open key 'A'.
    let a_open_buf = bincode::serialize(&a_open);
    if let Err(e) = a_open_buf {
        return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
    }
    let mut a_open_buf = a_open_buf.unwrap();

    // Send open key 'A'.
    let a_open_len = a_open_buf.len() as u64;
    let a_open_len_buf = bincode::serialize(&a_open_len);
    if let Err(e) = a_open_len_buf {
        return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
    }
    let mut a_open_len_buf = a_open_len_buf.unwrap();
    a_open_len_buf.append(&mut a_open_buf);
    match write_to_socket(socket, &mut a_open_len_buf, true) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received", file!(), line!()));
        }
        IoResult::Err(err) => {
            return Err(err.add_entry(file!(), line!()));
        }
        IoResult::Ok(_) => {}
    }

    // Receive open key 'B' size.
    let mut b_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    match read_from_socket(socket, &mut b_open_len_buf) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received", file!(), line!()));
        }
        IoResult::Err(err) => {
            return Err(err.add_entry(file!(), line!()));
        }
        IoResult::Ok(_) => {}
    }

    // Receive open key 'B'.
    let b_open_len = bincode::deserialize::<u64>(&b_open_len_buf);
    if let Err(e) = b_open_len {
        return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
    }
    let b_open_len = b_open_len.unwrap();
    let mut b_open_buf = vec![0u8; b_open_len as usize];

    match read_from_socket(socket, &mut b_open_buf) {
        IoResult::Fin => {
            return Err(AppError::new("unexpected FIN received", file!(), line!()));
        }
        IoResult::Err(err) => {
            return Err(err.add_entry(file!(), line!()));
        }
        IoResult::Ok(_) => {}
    }

    let b_open_big = bincode::deserialize::<BigUint>(&b_open_buf);
    if let Err(e) = b_open_big {
        return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
    }
    let b_open_big = b_open_big.unwrap();

    // Calculate the secret key.
    let secret_key = b_open_big.modpow(&a, &p);
    let mut secret_key_str = secret_key.to_str_radix(10);

    if secret_key_str.len() < SECRET_KEY_SIZE {
        if secret_key_str.is_empty() {
            return Err(AppError::new(
                "generated secret key is empty",
                file!(),
                line!(),
            ));
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
    loop {
        match read_from_socket(socket, &mut p_len_buf) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }
    let p_len = bincode::deserialize::<u64>(&p_len_buf);
    if let Err(e) = p_len {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let p_len = p_len.unwrap();

    // Get 'p' value.
    let mut p_buf = vec![0u8; p_len as usize];
    loop {
        match read_from_socket(socket, &mut p_buf) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }
    let p_buf = bincode::deserialize::<BigUint>(&p_buf);
    if let Err(e) = p_buf {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let p = p_buf.unwrap();

    // Get 'g' len.
    let mut g_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    loop {
        match read_from_socket(socket, &mut g_len_buf) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }
    let g_len = bincode::deserialize::<u64>(&g_len_buf);
    if let Err(e) = g_len {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let g_len = g_len.unwrap();

    // Get 'g' value.
    let mut g_buf = vec![0u8; g_len as usize];
    loop {
        match read_from_socket(socket, &mut g_buf) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }
    let g_buf = bincode::deserialize::<BigUint>(&g_buf);
    if let Err(e) = g_buf {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let g = g_buf.unwrap();

    // Calculate the open key B.
    let b_open = g.modpow(&b, &p);

    // Receive the open key A size.
    let mut a_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
    loop {
        match read_from_socket(socket, &mut a_open_len_buf) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }

    let a_open_len = bincode::deserialize::<u64>(&a_open_len_buf);
    if let Err(e) = a_open_len {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let a_open_len = a_open_len.unwrap();

    // Receive the open key A.
    let mut a_open_buf = vec![0u8; a_open_len as usize];
    loop {
        match read_from_socket(socket, &mut a_open_buf) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }

    let a_open_big = bincode::deserialize::<BigUint>(&a_open_buf);
    if let Err(e) = a_open_big {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let a_open_big = a_open_big.unwrap();

    // Prepare to send open key B.
    let mut b_open_buf = bincode::serialize(&b_open).unwrap();

    // Send open key 'B'.
    let b_open_len = b_open_buf.len() as u64;
    let b_open_len_buf = bincode::serialize(&b_open_len);
    if let Err(e) = b_open_len_buf {
        return Err(AppError::new(&e.to_string(), file!(), line!()));
    }
    let mut b_open_len_buf = b_open_len_buf.unwrap();
    b_open_len_buf.append(&mut b_open_buf);
    loop {
        match write_to_socket(socket, &mut b_open_len_buf, true) {
            IoResult::Fin => {
                return Err(AppError::new("unexpected FIN received", file!(), line!()));
            }
            IoResult::Err(app_error) => {
                return Err(app_error.add_entry(file!(), line!()));
            }
            IoResult::Ok(_) => {
                break;
            }
        }
    }

    // Calculate the secret key.
    let secret_key = a_open_big.modpow(&b, &p);
    let mut secret_key_str = secret_key.to_str_radix(10);

    if secret_key_str.len() < SECRET_KEY_SIZE {
        if secret_key_str.is_empty() {
            return Err(AppError::new(
                "generated secret key is empty",
                file!(),
                line!(),
            ));
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
            file!(),
            line!(),
        ));
    }

    // Serialize.
    let mut binary_message = bincode::serialize(&message).unwrap();

    // CMAC.
    let mut mac = Cmac::<Aes256>::new_from_slice(secret_key).unwrap();
    mac.update(&binary_message);
    let result = mac.finalize();
    let mut tag_bytes = result.into_bytes().to_vec();
    if tag_bytes.len() != CMAC_TAG_LENGTH {
        return Some(AppError::new(
            &format!(
                "unexpected tag length: {} != {}",
                tag_bytes.len(),
                CMAC_TAG_LENGTH
            ),
            file!(),
            line!(),
        ));
    }

    binary_message.append(&mut tag_bytes);

    // Encrypt message.
    let mut rng = rand::thread_rng();
    let mut iv = [0u8; IV_LENGTH];
    rng.fill_bytes(&mut iv);
    let mut encrypted_binary_message = Aes256CbcEnc::new(secret_key.into(), &iv.into())
        .encrypt_padded_vec_mut::<Pkcs7>(&binary_message);

    // Prepare encrypted message len buffer.
    if encrypted_binary_message.len() + IV_LENGTH > std::u32::MAX as usize {
        // should never happen
        return Some(AppError::new(
            &format!(
                "resulting message is too big ({} > {})",
                encrypted_binary_message.len() + IV_LENGTH,
                std::u32::MAX
            ),
            file!(),
            line!(),
        ));
    }
    let encrypted_len = (encrypted_binary_message.len() + IV_LENGTH) as u32;
    let encrypted_len_buf = bincode::serialize(&encrypted_len);
    if let Err(e) = encrypted_len_buf {
        return Some(AppError::new(&format!("{:?}", e), file!(), line!()));
    }
    let mut send_buffer = encrypted_len_buf.unwrap();

    // Merge all to one buffer.
    send_buffer.append(&mut Vec::from(iv));
    send_buffer.append(&mut encrypted_binary_message);

    // Send.
    match write_to_socket(socket, &mut send_buffer, true) {
        IoResult::Fin => {
            return Some(AppError::new("unexpected FIN received", file!(), line!()));
        }
        IoResult::Err(err) => return Some(err.add_entry(file!(), line!())),
        IoResult::Ok(_) => {}
    }

    None
}

/// Waits for next message to arrive.
///
/// Parameters:
/// - `socket`: socket to use.
/// - `secret_key`: secret key to decrypt message.
/// - `timeout_in_ms`: if specified the operation will have a timeout.
/// - `max_allowed_message_size_in_bytes`: maximum size of allowed message, if
/// the incoming message is bigger than this value an error will be returned.
/// - `is_fin`: will be `true` if the remote socket closed connection.
/// if we reached timeout and this parameter was not `None` will return `Ok` with
/// zero length vector.
pub fn receive_message(
    socket: &mut TcpStream,
    secret_key: &[u8; SECRET_KEY_SIZE],
    timeout_in_ms: Option<u64>,
    max_allowed_message_size_in_bytes: u64,
    is_fin: &mut bool,
) -> Result<Vec<u8>, AppError> {
    if secret_key.is_empty() {
        return Err(AppError::new(
            "can't receive message - secure connected is not established",
            file!(),
            line!(),
        ));
    }

    // Get socket remote address.
    let peer_addr = socket.peer_addr();
    if let Err(e) = peer_addr {
        return Err(AppError::new(
            &format!("failed to get socket peer address (error: {})", e),
            file!(),
            line!(),
        ));
    }
    let socket_addr = peer_addr.unwrap();

    // Read u32 (size of a packet)
    let mut packet_size_buf = [0u8; std::mem::size_of::<u32>() as usize];
    let mut _next_packet_size: u64 = 0;

    let mut _result = IoResult::Fin;
    if timeout_in_ms.is_some() {
        let timeout_result =
            read_from_socket_with_timeout(socket, &mut packet_size_buf, timeout_in_ms.unwrap());
        if timeout_result.is_none() {
            return Ok(Vec::new());
        } else {
            _result = timeout_result.unwrap();
        }
    } else {
        _result = read_from_socket(socket, &mut packet_size_buf);
    }

    match _result {
        IoResult::Fin => {
            *is_fin = true;
            return Err(AppError::new(
                &format!("unexpected FIN received (socket: {})", socket_addr),
                file!(),
                line!(),
            ));
        }
        IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
        IoResult::Ok(byte_count) => {
            if byte_count != packet_size_buf.len() {
                return Err(AppError::new(
                    &format!(
                        "not all data received (got: {}, expected: {}) (socket: {})",
                        byte_count,
                        packet_size_buf.len(),
                        socket_addr
                    ),
                    file!(),
                    line!(),
                ));
            }

            let res = bincode::deserialize::<u32>(&packet_size_buf);
            if let Err(e) = res {
                return Err(AppError::new(
                    &format!("{:?} (socket: {})", e, socket_addr),
                    file!(),
                    line!(),
                ));
            }

            _next_packet_size = res.unwrap() as u64;
        }
    }

    // Check packet size.
    if _next_packet_size > max_allowed_message_size_in_bytes {
        return Err(AppError::new(
            &format!(
                "incoming message is too big to receive ({} > {} bytes) (socket: {})",
                _next_packet_size, max_allowed_message_size_in_bytes, socket_addr
            ),
            file!(),
            line!(),
        ));
    }

    // Receive encrypted packet.
    let mut encrypted_packet = vec![0u8; _next_packet_size as usize];
    match read_from_socket(socket, &mut encrypted_packet) {
        IoResult::Fin => {
            *is_fin = true;
            return Err(AppError::new(
                &format!("unexpected FIN received (socket: {})", socket_addr),
                file!(),
                line!(),
            ));
        }
        IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
        IoResult::Ok(_) => {}
    };

    // Get IV.
    if encrypted_packet.len() < IV_LENGTH {
        return Err(AppError::new(
            &format!(
                "unexpected packet length ({}) (socket: {})",
                encrypted_packet.len(),
                socket_addr
            ),
            file!(),
            line!(),
        ));
    }
    let iv = encrypted_packet[..IV_LENGTH].to_vec();
    encrypted_packet = encrypted_packet[IV_LENGTH..].to_vec();

    // Convert IV.
    let iv = iv.try_into();
    if iv.is_err() {
        return Err(AppError::new(
            "failed to convert iv to generic array",
            file!(),
            line!(),
        ));
    }
    let iv: [u8; IV_LENGTH] = iv.unwrap();

    // Decrypt packet.
    let decrypted_packet = Aes256CbcDec::new(secret_key.into(), &iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(&encrypted_packet);
    if let Err(e) = decrypted_packet {
        return Err(AppError::new(
            &format!("{:?} (socket: {})", e, socket_addr),
            file!(),
            line!(),
        ));
    }
    let mut decrypted_packet = decrypted_packet.unwrap();

    // CMAC
    let mut mac = Cmac::<Aes256>::new_from_slice(secret_key).unwrap();
    let tag: Vec<u8> = decrypted_packet
        .drain(decrypted_packet.len().saturating_sub(CMAC_TAG_LENGTH)..)
        .collect();
    mac.update(&decrypted_packet);

    // Convert tag.
    let tag = tag.try_into();
    if tag.is_err() {
        return Err(AppError::new(
            "failed to convert cmac tag to generic array",
            file!(),
            line!(),
        ));
    }
    let tag: [u8; CMAC_TAG_LENGTH] = tag.unwrap();

    // Check that tag is correct.
    if let Err(e) = mac.verify(&tag.into()) {
        return Err(AppError::new(
            &format!("{:?} (socket: {})", e, socket_addr),
            file!(),
            line!(),
        ));
    }

    Ok(decrypted_packet)
}

/// Writes the specified buffer to the socket.
///
/// Parameters:
/// - `socket`: socket to write this data to.
/// - `buf`: buffer to write to the socket.
/// - `enable_wait_limit`: if `false` will wait for write operation to finish
/// infinitely, otherwise will wait for maximum `MAX_WAIT_TIME_IN_READ_WRITE_MS`
/// for operation to finish and return error in case of a timeout.
fn write_to_socket(socket: &mut TcpStream, buf: &mut [u8], enable_wait_limit: bool) -> IoResult {
    if buf.is_empty() {
        return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
    }

    let mut total_wait_time_ms: u64 = 0;

    loop {
        if enable_wait_limit && total_wait_time_ms >= MAX_WAIT_TIME_IN_READ_WRITE_MS {
            return IoResult::Err(AppError::new(
                &format!(
                    "reached maximum response wait time limit of {} ms for socket {}",
                    MAX_WAIT_TIME_IN_READ_WRITE_MS,
                    match socket.peer_addr() {
                        Ok(addr) => {
                            addr.to_string()
                        }
                        Err(_) => {
                            String::new()
                        }
                    },
                ),
                file!(),
                line!(),
            ));
        }

        match socket.write(buf) {
            Ok(0) => {
                return IoResult::Fin;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(AppError::new(
                        &format!(
                            "failed to write (got: {}, expected: {}) (socket {})",
                            n,
                            buf.len(),
                            match socket.peer_addr() {
                                Ok(addr) => {
                                    addr.to_string()
                                }
                                Err(_) => {
                                    String::new()
                                }
                            },
                        ),
                        file!(),
                        line!(),
                    ));
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                continue;
            }
            Err(e) => {
                return IoResult::Err(AppError::new(
                    &format!(
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
                    ),
                    file!(),
                    line!(),
                ));
            }
        };
    }
}

/// Reads data from the specified socket.
/// Blocks the current thread until the data is received or timeout is reached.
///
/// Arguments:
/// - `socket`: socket to read the data from.
/// - `buf`: buffer to write read data.
///
/// Returns `None` if timeout reached.
fn read_from_socket_with_timeout(
    socket: &mut TcpStream,
    buf: &mut [u8],
    timeout_in_ms: u64,
) -> Option<IoResult> {
    if buf.is_empty() {
        return Some(IoResult::Err(AppError::new(
            "passed 'buf' has 0 length",
            file!(),
            line!(),
        )));
    }

    let mut total_wait_time_ms: u64 = 0;

    loop {
        if total_wait_time_ms >= timeout_in_ms {
            return None;
        }

        match socket.read(buf) {
            Ok(0) => {
                return Some(IoResult::Fin);
            }
            Ok(n) => {
                if n != buf.len() {
                    return Some(IoResult::Err(AppError::new(
                        &format!(
                            "failed to read (got: {}, expected: {}) (socket {})",
                            n,
                            buf.len(),
                            match socket.peer_addr() {
                                Ok(addr) => {
                                    addr.to_string()
                                }
                                Err(_) => {
                                    String::new()
                                }
                            },
                        ),
                        file!(),
                        line!(),
                    )));
                }

                return Some(IoResult::Ok(n));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                continue;
            }
            Err(e) => {
                return Some(IoResult::Err(AppError::new(
                    &format!(
                        "{:?} (socket {})",
                        e,
                        match socket.peer_addr() {
                            Ok(addr) => {
                                addr.to_string()
                            }
                            Err(_) => {
                                String::new()
                            }
                        },
                    ),
                    file!(),
                    line!(),
                )));
            }
        };
    }
}

/// Reads data from the specified socket.
/// Infinitely blocks the current thread until the data is received.
///
/// Parameters:
/// - `socket`: socket to read the data from.
/// - `buf`: buffer to write read data.
fn read_from_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
    if buf.is_empty() {
        return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
    }

    let mut total_wait_time_ms: u64 = 0;

    loop {
        if total_wait_time_ms >= MAX_WAIT_TIME_IN_READ_WRITE_MS {
            return IoResult::Err(AppError::new(
                &format!(
                    "reached maximum response wait time limit of {} ms for socket {}",
                    MAX_WAIT_TIME_IN_READ_WRITE_MS,
                    match socket.peer_addr() {
                        Ok(addr) => {
                            addr.to_string()
                        }
                        Err(_) => {
                            String::new()
                        }
                    },
                ),
                file!(),
                line!(),
            ));
        }

        match socket.read(buf) {
            Ok(0) => {
                return IoResult::Fin;
            }
            Ok(n) => {
                if n != buf.len() {
                    return IoResult::Err(AppError::new(
                        &format!(
                            "failed to read (got: {}, expected: {}) (socket {})",
                            n,
                            buf.len(),
                            match socket.peer_addr() {
                                Ok(addr) => {
                                    addr.to_string()
                                }
                                Err(_) => {
                                    String::new()
                                }
                            },
                        ),
                        file!(),
                        line!(),
                    ));
                }

                return IoResult::Ok(n);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                continue;
            }
            Err(e) => {
                return IoResult::Err(AppError::new(
                    &format!(
                        "{:?} (socket {})",
                        e,
                        match socket.peer_addr() {
                            Ok(addr) => {
                                addr.to_string()
                            }
                            Err(_) => {
                                String::new()
                            }
                        },
                    ),
                    file!(),
                    line!(),
                ));
            }
        };
    }
}