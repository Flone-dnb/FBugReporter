// Std.
use std::io::prelude::*;
use std::net::*;
use std::thread;
use std::time::Duration;

// External.
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes256;
use cmac::{Cmac, Mac};
use num_bigint::{BigUint, RandomBits};
use rand::{Rng, RngCore};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
const SECRET_KEY_SIZE: usize = 32; // if changed, change protocol version

// Custom.
use crate::logger_service::Logger;
use crate::net_packets::InPacket;
use crate::net_packets::OutPacket;
use shared::report::*;

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 10;

pub const NETWORK_PROTOCOL_VERSION: u16 = 1;

enum IoResult {
    Ok(usize),
    Fin,
    Err(String),
}

pub struct ReporterService {
    tcp_socket: Option<TcpStream>,
}

impl ReporterService {
    pub fn new() -> Self {
        Self { tcp_socket: None }
    }
    pub fn send_report(
        &mut self,
        server_addr: SocketAddrV4,
        report: GameReport,
        logger: &mut Logger,
        attachments: Vec<ReportAttachment>,
    ) -> (ReportResult, Option<String>) {
        let tcp_socket = TcpStream::connect(server_addr);

        if let Err(e) = tcp_socket {
            return (ReportResult::CouldNotConnect, Some(format!("{:?}", e)));
        } else {
            logger.log("Connected to the server.");
        }

        let tcp_socket = tcp_socket.unwrap();
        if let Err(e) = tcp_socket.set_nodelay(true) {
            return (
                ReportResult::InternalError,
                Some(format!(
                    "An error occurred at [{}, {}]: {:?}",
                    file!(),
                    line!(),
                    e
                )),
            );
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return (
                ReportResult::InternalError,
                Some(format!(
                    "An error occurred at [{}, {}]: {:?}",
                    file!(),
                    line!(),
                    e
                )),
            );
        }
        self.tcp_socket = Some(tcp_socket);

        let secret_key = self.establish_secure_connection();
        if let Err(msg) = secret_key {
            return (
                ReportResult::InternalError,
                Some(format!("{} at [{}, {}]\n\n", msg, file!(), line!())),
            );
        } else {
            logger.log("Secure connection established.");
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return (
                ReportResult::InternalError,
                Some(format!(
                    "failed to convert Vec<u8> to generic array at [{}, {}]\n\n",
                    file!(),
                    line!()
                )),
            );
        }
        let secret_key: [u8; SECRET_KEY_SIZE] = result.unwrap();

        // Prepare packet.
        let packet = OutPacket::ReportPacket {
            reporter_net_protocol: NETWORK_PROTOCOL_VERSION,
            game_report: report,
            attachments,
        };

        // Serialize.
        let mut binary_packet = bincode::serialize(&packet).unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&secret_key).unwrap();
        mac.update(&binary_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return (
                ReportResult::InternalError,
                Some(format!(
                    "An error occurred at [{}, {}]: unexpected tag length: {} != {}.",
                    file!(),
                    line!(),
                    tag_bytes.len(),
                    CMAC_TAG_LENGTH
                )),
            );
        }

        binary_packet.append(&mut tag_bytes);

        // Encrypt packet.
        let mut rng = rand::thread_rng();
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_packet = Aes256CbcEnc::new(&secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_packet);

        // Prepare encrypted packet len buffer.
        if encrypted_packet.len() + IV_LENGTH > std::u32::MAX as usize {
            // should never happen
            return (
                ReportResult::InternalError,
                Some(format!(
                    "An error occurred at [{}, {}]: resulting packet is too big ({} > {})",
                    file!(),
                    line!(),
                    encrypted_packet.len() + IV_LENGTH,
                    std::u32::MAX
                )),
            );
        }
        let encrypted_len = (encrypted_packet.len() + IV_LENGTH) as u32;
        let encrypted_len_buf = bincode::serialize(&encrypted_len);
        if let Err(e) = encrypted_len_buf {
            return (
                ReportResult::InternalError,
                Some(format!(
                    "An error occurred at [{}, {}]: {:?}",
                    file!(),
                    line!(),
                    e
                )),
            );
        }
        let mut send_buffer = encrypted_len_buf.unwrap();

        // Merge all to one buffer.
        send_buffer.append(&mut Vec::from(iv));
        send_buffer.append(&mut encrypted_packet);

        // Send to the server.
        loop {
            match self.write_to_socket(&mut send_buffer) {
                IoResult::Fin => {
                    return (
                        ReportResult::InternalError,
                        Some(format!(
                            "An error occurred at [{}, {}]: unexpected FIN received.",
                            file!(),
                            line!()
                        )),
                    );
                }
                IoResult::Err(msg) => {
                    return (
                        ReportResult::InternalError,
                        Some(format!("{} at [{}, {}]\n\n", msg, file!(), line!())),
                    );
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let result = self.receive_answer(&secret_key);
        if let Err(msg) = result {
            // We failed somewhere in the receive_answer().
            return (
                ReportResult::InternalError,
                Some(format!("{} at [{}, {}]\n\n", msg, file!(), line!())),
            );
        }

        let result_code = result.unwrap();
        if result_code != ReportResult::Ok {
            // Server returned an error.
            return (
                result_code,
                Some(format!("The server returned error: {:?}", result_code)),
            );
        }

        return (result_code, None);
    }
    fn receive_answer(
        &mut self,
        secret_key: &[u8; SECRET_KEY_SIZE],
    ) -> Result<ReportResult, String> {
        if secret_key.is_empty() {
            return Err(format!(
                "An error occurred at [{}, {}]: secure connected is not established - can't receive a packet.",
                file!(),
                line!(),
            ));
        }

        // Read u32 (size of a packet)
        let mut packet_size_buf = [0u8; std::mem::size_of::<u32>() as usize];
        let mut _next_packet_size: u32 = 0;
        match self.read_from_socket(&mut packet_size_buf) {
            IoResult::Fin => {
                return Err(format!(
                    "An error occurred at [{}, {}]: unexpected FIN received.",
                    file!(),
                    line!(),
                ));
            }
            IoResult::Err(msg) => {
                return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!(),));
            }
            IoResult::Ok(byte_count) => {
                if byte_count != packet_size_buf.len() {
                    return Err(format!(
                        "An error occurred at [{}, {}]: not all data received (got: {}, expected: {}).",
                        file!(),
                        line!(),
                        byte_count,
                        packet_size_buf.len(),
                    ));
                }

                let res = bincode::deserialize(&packet_size_buf);
                if let Err(e) = res {
                    return Err(format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e,
                    ));
                }

                _next_packet_size = res.unwrap();
            }
        }

        // Receive encrypted packet.
        let mut encrypted_packet = vec![0u8; _next_packet_size as usize];
        match self.read_from_socket(&mut encrypted_packet) {
            IoResult::Fin => {
                return Err(format!(
                    "An error occurred at [{}, {}]: unexpected FIN received.",
                    file!(),
                    line!(),
                ));
            }
            IoResult::Err(msg) => {
                return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!(),));
            }
            IoResult::Ok(_) => {}
        };

        // Get IV.
        if encrypted_packet.len() < IV_LENGTH {
            return Err(format!(
                "An error occurred at [{}, {}]: unexpected packet length ({}).",
                file!(),
                line!(),
                encrypted_packet.len(),
            ));
        }
        let iv = encrypted_packet[..IV_LENGTH].to_vec();
        encrypted_packet = encrypted_packet[IV_LENGTH..].to_vec();

        // Convert IV.
        let iv = iv.try_into();
        if iv.is_err() {
            return Err(format!(
                "An error occurred at [{}, {}]: failed to convert iv to generic array\n\n",
                file!(),
                line!(),
            ));
        }
        let iv: [u8; IV_LENGTH] = iv.unwrap();

        // Decrypt packet.
        let decrypted_packet = Aes256CbcDec::new(secret_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&encrypted_packet);
        if let Err(e) = decrypted_packet {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e,
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
            return Err(format!(
                "An error occurred at [{}, {}]: failed to convert cmac tag to generic array\n\n",
                file!(),
                line!(),
            ));
        }
        let tag: [u8; CMAC_TAG_LENGTH] = tag.unwrap();

        // Check that tag is correct.
        if let Err(e) = mac.verify(&tag.into()) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e,
            ));
        }

        // Deserialize.
        let packet = bincode::deserialize::<InPacket>(&decrypted_packet);
        if let Err(e) = packet {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e,
            ));
        }

        let packet = packet.unwrap();
        match packet {
            InPacket::ReportAnswer { result_code } => Ok(result_code),
        }
    }
    fn establish_secure_connection(&mut self) -> Result<Vec<u8>, String> {
        // Generate secret key 'b'.
        let mut rng = rand::thread_rng();
        let b: BigUint = rng.sample(RandomBits::new(A_B_BITS));

        // Receive 2 values: p (BigUint), g (BigUint) values.
        // Get 'p' len.
        let mut p_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut p_len_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let p_len = bincode::deserialize::<u64>(&p_len_buf);
        if let Err(e) = p_len {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let p_len = p_len.unwrap();

        // Get 'p' value.
        let mut p_buf = vec![0u8; p_len as usize];
        loop {
            match self.read_from_socket(&mut p_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let p_buf = bincode::deserialize::<BigUint>(&p_buf);
        if let Err(e) = p_buf {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let p = p_buf.unwrap();

        // Get 'g' len.
        let mut g_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut g_len_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let g_len = bincode::deserialize::<u64>(&g_len_buf);
        if let Err(e) = g_len {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let g_len = g_len.unwrap();

        // Get 'g' value.
        let mut g_buf = vec![0u8; g_len as usize];
        loop {
            match self.read_from_socket(&mut g_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }
        let g_buf = bincode::deserialize::<BigUint>(&g_buf);
        if let Err(e) = g_buf {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let g = g_buf.unwrap();

        // Calculate the open key B.
        let b_open = g.modpow(&b, &p);

        // Receive the open key A size.
        let mut a_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut a_open_len_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let a_open_len = bincode::deserialize::<u64>(&a_open_len_buf);
        if let Err(e) = a_open_len {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let a_open_len = a_open_len.unwrap();

        // Receive the open key A.
        let mut a_open_buf = vec![0u8; a_open_len as usize];
        loop {
            match self.read_from_socket(&mut a_open_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let a_open_big = bincode::deserialize::<BigUint>(&a_open_buf);
        if let Err(e) = a_open_big {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let a_open_big = a_open_big.unwrap();

        // Prepare to send open key B.
        let mut b_open_buf = bincode::serialize(&b_open).unwrap();

        // Send open key 'B'.
        let b_open_len = b_open_buf.len() as u64;
        let b_open_len_buf = bincode::serialize(&b_open_len);
        if let Err(e) = b_open_len_buf {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let mut b_open_len_buf = b_open_len_buf.unwrap();
        b_open_len_buf.append(&mut b_open_buf);
        loop {
            match self.write_to_socket(&mut b_open_len_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.\n\n",
                        file!(),
                        line!()
                    ));
                }
                IoResult::Err(msg) => {
                    return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
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
                return Err(format!(
                    "An error occurred at [{}, {}]: generated secret key is empty.\n\n",
                    file!(),
                    line!()
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
    fn read_from_socket(&mut self, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(format!(
                "An error occurred at [{}, {}]: passed 'buf' has 0 length.\n\n",
                file!(),
                line!()
            ));
        }

        if self.tcp_socket.is_none() {
            return IoResult::Err(format!(
                "An error occurred at [{}, {}]: the socket is None.\n\n",
                file!(),
                line!()
            ));
        }

        loop {
            match self.tcp_socket.as_mut().unwrap().read(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(format!(
                            "An error occurred at [{}, {}]: failed to read (got: {}, expected: {})",
                            file!(),
                            line!(),
                            n,
                            buf.len()
                        ));
                    }

                    return IoResult::Ok(n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    continue;
                }
                Err(e) => {
                    return IoResult::Err(format!(
                        "An error occurred at [{}, {}]: {:?}.\n\n",
                        file!(),
                        line!(),
                        e
                    ));
                }
            };
        }
    }
    fn write_to_socket(&mut self, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(format!(
                "An error occurred at [{}, {}]: passed 'buf' has 0 length.\n\n",
                file!(),
                line!()
            ));
        }

        if self.tcp_socket.is_none() {
            return IoResult::Err(format!(
                "An error occurred at [{}, {}]: the socket is None.\n\n",
                file!(),
                line!()
            ));
        }

        loop {
            match self.tcp_socket.as_mut().unwrap().write(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(format!(
                            "An error occurred at [{}, {}]: failed to write (got: {}, expected: {})",
                            file!(),
                            line!(),
                            n,
                            buf.len()
                        ));
                    }

                    return IoResult::Ok(n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    continue;
                }
                Err(e) => {
                    return IoResult::Err(format!(
                        "An error occurred at [{}, {}]: {:?}.\n\n",
                        file!(),
                        line!(),
                        e
                    ));
                }
            };
        }
    }
}
