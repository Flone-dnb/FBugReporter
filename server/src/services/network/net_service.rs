// Std.
use std::io::prelude::*;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// External.
use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use cmac::{Cmac, Mac, NewMac};
use num_bigint::{BigUint, RandomBits};
use rand::Rng;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;
const KEY_LENGTH_IN_BYTES: usize = 32; // if changed, change protocol version

// Custom.
use super::net_packets::ReportPacket;
use crate::services::config_service::ServerConfig;
use crate::services::logger_service::Logger;

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const NETWORK_PROTOCOL_VERSION: u16 = 0;
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 10;
pub const SERVER_PASSWORD_BIT_COUNT: u64 = 1024;

enum IoResult {
    Ok(usize),
    Fin,
    Err(String),
}

pub struct NetService {
    pub logger: Arc<Mutex<Logger>>,
    pub server_config: ServerConfig,
}

impl NetService {
    pub fn new(logger: Logger) -> Result<Self, String> {
        let config = ServerConfig::new();
        if let Err(e) = config {
            return Err(format!("{} at [{}, {}]\n\n", e, file!(), line!()));
        }

        Ok(Self {
            server_config: config.unwrap(),
            logger: Arc::new(Mutex::new(logger)),
        })
    }
    pub fn refresh_password(&mut self) -> Result<(), String> {
        if let Err(msg) = self.server_config.refresh_password() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn refresh_port(&mut self) -> Result<(), String> {
        if let Err(msg) = self.server_config.refresh_port() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn set_port(&mut self, port: u16) -> Result<(), String> {
        if let Err(msg) = self.server_config.set_port(port) {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn start(&self) {
        {
            self.logger.lock().unwrap().print_and_log("Starting...");
        }

        // Create socket.
        let listener_socker =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.server_port));
        if let Err(ref e) = listener_socker {
            self.logger.lock().unwrap().print_and_log(&format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let listener_socket = listener_socker.unwrap();

        {
            self.logger.lock().unwrap().print_and_log(&format!(
                "Ready to accept connections on port {}",
                self.server_config.server_port
            ));
        }

        loop {
            // Wait for connection.
            let accept_result = listener_socket.accept();
            if let Err(ref e) = accept_result {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }

            let (mut socket, addr) = accept_result.unwrap();
            {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "Accepted connection with {}:{}.",
                    addr.ip(),
                    addr.port()
                ));
            }

            if let Err(e) = socket.set_nodelay(true) {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }
            if let Err(e) = socket.set_nonblocking(true) {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }

            let secret_key = NetService::establish_secure_connection(&mut socket);
            if let Err(msg) = secret_key {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "{} at [{}, {}] (socket: {}:{}).\n\n",
                    msg,
                    file!(),
                    line!(),
                    addr.ip(),
                    addr.port(),
                ));
                continue;
            }
            let secret_key = secret_key.unwrap();

            // Read u32 (size of a packet)
            let mut packet_size_buf = [0u8; std::mem::size_of::<u32>() as usize];
            let mut _next_packet_size: u32 = 0;
            match NetService::read_from_socket(&mut socket, &mut packet_size_buf) {
                IoResult::Fin => {
                    self.logger.lock().unwrap().print_and_log(&format!(
                        "An error occurred at [{}, {}]: unexpected FIN received (socket: {}:{}).",
                        file!(),
                        line!(),
                        addr.ip(),
                        addr.port(),
                    ));
                    continue;
                }
                IoResult::Err(msg) => {
                    self.logger.lock().unwrap().print_and_log(&format!(
                        "{} at [{}, {}] (socket: {}:{})\n\n",
                        msg,
                        file!(),
                        line!(),
                        addr.ip(),
                        addr.port(),
                    ));
                    continue;
                }
                IoResult::Ok(byte_count) => {
                    if byte_count != packet_size_buf.len() {
                        self.logger.lock().unwrap().print_and_log(&format!(
                            "An error occurred at [{}, {}]: not all data received (got: {}, expected: {}) (socket: {}:{}).",
                            file!(),
                            line!(),
                            byte_count,
                            packet_size_buf.len(),
                            addr.ip(),
                            addr.port(),
                        ));
                        continue;
                    }

                    let res = bincode::deserialize(&packet_size_buf);
                    if let Err(e) = res {
                        self.logger.lock().unwrap().print_and_log(&format!(
                            "An error occurred at [{}, {}]: {:?} (socket: {}:{})\n\n",
                            file!(),
                            line!(),
                            e,
                            addr.ip(),
                            addr.port(),
                        ));
                        continue;
                    }

                    _next_packet_size = res.unwrap();
                }
            }

            // Receive encrypted packet.
            let mut encrypted_packet = vec![0u8; _next_packet_size as usize];
            match NetService::read_from_socket(&mut socket, &mut encrypted_packet) {
                IoResult::Fin => {
                    self.logger.lock().unwrap().print_and_log(&format!(
                        "An error occurred at [{}, {}]: unexpected FIN received (socket: {}:{}).",
                        file!(),
                        line!(),
                        addr.ip(),
                        addr.port(),
                    ));
                    continue;
                }
                IoResult::Err(msg) => {
                    self.logger.lock().unwrap().print_and_log(&format!(
                        "{} at [{}, {}] (socket: {}:{})\n\n",
                        msg,
                        file!(),
                        line!(),
                        addr.ip(),
                        addr.port(),
                    ));
                    continue;
                }
                IoResult::Ok(_) => {}
            };

            // Get IV.
            if encrypted_packet.len() < IV_LENGTH {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: unexpected packet length ({}) (socket: {}:{}).",
                    file!(),
                    line!(),
                    encrypted_packet.len(),
                    addr.ip(),
                    addr.port(),
                ));
                continue;
            }
            let iv = &encrypted_packet[..IV_LENGTH].to_vec();
            encrypted_packet = encrypted_packet[IV_LENGTH..].to_vec();

            // Decrypt packet.
            let cipher = Aes256Cbc::new_from_slices(&secret_key, &iv).unwrap();
            let decrypted_packet = cipher.decrypt_vec(&encrypted_packet);
            if let Err(e) = decrypted_packet {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?} (socket: {}:{})\n\n",
                    file!(),
                    line!(),
                    e,
                    addr.ip(),
                    addr.port(),
                ));
                continue;
            }
            let mut decrypted_packet = decrypted_packet.unwrap();

            // CMAC
            let mut mac = Cmac::<Aes256>::new_from_slice(&secret_key).unwrap();
            let tag: Vec<u8> = decrypted_packet
                .drain(decrypted_packet.len().saturating_sub(CMAC_TAG_LENGTH)..)
                .collect();
            mac.update(&decrypted_packet);
            if let Err(e) = mac.verify(&tag) {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?} (socket: {}:{})\n\n",
                    file!(),
                    line!(),
                    e,
                    addr.ip(),
                    addr.port(),
                ));
                continue;
            }

            // Deserialize.
            let packet = bincode::deserialize::<ReportPacket>(&decrypted_packet);
            if let Err(e) = packet {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?} (socket: {}:{})\n\n",
                    file!(),
                    line!(),
                    e,
                    addr.ip(),
                    addr.port(),
                ));
                continue;
            }
            let packet = packet.unwrap();

            // Check protocol version.
            if packet.reporter_net_protocol != NETWORK_PROTOCOL_VERSION {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: wrong protocol version ({} != {}) (socket: {}:{})\n\n",
                    file!(),
                    line!(),
                    packet.reporter_net_protocol,
                    NETWORK_PROTOCOL_VERSION,
                    addr.ip(),
                    addr.port(),
                ));
                continue;
            }

            {
                self.logger
                    .lock()
                    .unwrap()
                    .print_and_log(&format!("Received report: {:?}", packet.game_report));
            }

            // TODO: move all this code to the new process_user() function.
            //       add a helper function receive_packet() with generics.
            // TODO: check protocol version (send ReportResult::WrongProtocol if different)
            // TODO: in wouldblock (read/write functions) have loop limit as a variable
            //       never set the limit when waiting for user messages!
            //       pass loop limit to read/write functions
            // TODO: add keep alive timer
            // TODO: send ReportResult::NetworkIssue if cmac or other errors
            // TODO: send ReportResult::ServerRejected if any fields exceed limits
            // TODO: (only for clients, not for reporters) check password hash and etc (send ReportResult::NetworkIssue if cmac or other errors)
        }
    }
    fn establish_secure_connection(socket: &mut TcpStream) -> Result<Vec<u8>, String> {
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
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.",
                file!(),
                line!(),
                e
            ));
        }
        let mut p_buf = p_buf.unwrap();

        if let Err(e) = g_buf {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.",
                file!(),
                line!(),
                e
            ));
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
        loop {
            match NetService::write_to_socket(socket, &mut pg_send_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.",
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

        // Generate secret key 'a'.
        let mut rng = rand::thread_rng();
        let a: BigUint = rng.sample(RandomBits::new(A_B_BITS));

        // Generate open key 'A'.
        let a_open = g.modpow(&a, &p);

        // Prepare to send open key 'A'.
        let a_open_buf = bincode::serialize(&a_open);
        if let Err(e) = a_open_buf {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.",
                file!(),
                line!(),
                e
            ));
        }
        let mut a_open_buf = a_open_buf.unwrap();

        // Send open key 'A'.
        let a_open_len = a_open_buf.len() as u64;
        let a_open_len_buf = bincode::serialize(&a_open_len);
        if let Err(e) = a_open_len_buf {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.",
                file!(),
                line!(),
                e
            ));
        }
        let mut a_open_len_buf = a_open_len_buf.unwrap();
        a_open_len_buf.append(&mut a_open_buf);
        loop {
            match NetService::write_to_socket(socket, &mut a_open_len_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.",
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

        // Receive open key 'B' size.
        let mut b_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match NetService::read_from_socket(socket, &mut b_open_len_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.",
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

        // Receive open key 'B'.
        let b_open_len = bincode::deserialize::<u64>(&b_open_len_buf);
        if let Err(e) = b_open_len {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.",
                file!(),
                line!(),
                e
            ));
        }
        let b_open_len = b_open_len.unwrap();
        let mut b_open_buf = vec![0u8; b_open_len as usize];

        loop {
            match NetService::read_from_socket(socket, &mut b_open_buf) {
                IoResult::Fin => {
                    return Err(format!(
                        "An error occurred at [{}, {}]: unexpected FIN received.",
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

        let b_open_big = bincode::deserialize::<BigUint>(&b_open_buf);
        if let Err(e) = b_open_big {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.",
                file!(),
                line!(),
                e
            ));
        }
        let b_open_big = b_open_big.unwrap();

        // Calculate the secret key.
        let secret_key = b_open_big.modpow(&a, &p);
        let mut secret_key_str = secret_key.to_str_radix(10);

        if secret_key_str.len() < KEY_LENGTH_IN_BYTES {
            if secret_key_str.is_empty() {
                return Err(format!(
                    "An error occurred at [{}, {}]: generated secret key is empty.",
                    file!(),
                    line!()
                ));
            }

            loop {
                secret_key_str += &secret_key_str.clone();

                if secret_key_str.len() >= KEY_LENGTH_IN_BYTES {
                    break;
                }
            }
        }

        Ok(Vec::from(&secret_key_str[0..KEY_LENGTH_IN_BYTES]))
    }
    fn read_from_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(format!(
                "An error occurred at [{}, {}]: passed 'buf' has 0 length.\n\n",
                file!(),
                line!()
            ));
        }

        loop {
            match socket.read(buf) {
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
    fn write_to_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(format!(
                "An error occurred at [{}, {}]: passed 'buf' has 0 length.\n\n",
                file!(),
                line!()
            ));
        }

        loop {
            match socket.write(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(format!(
                            "An error occurred at [{}, {}]: failed to write (got: {}, expected: {}).",
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
