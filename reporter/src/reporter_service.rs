// Std.
use std::io::prelude::*;
use std::net::*;
use std::thread;
use std::time::Duration;

// External.
use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use cmac::{Cmac, Mac, NewMac};
use num_bigint::{BigUint, RandomBits};
use rand::{Rng, RngCore};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;
const KEY_LENGTH_IN_BYTES: usize = 32; // if changed, change protocol version

// Custom.
use crate::logger_service::Logger;
use crate::misc::{GameReport, ReportResult};
use crate::net_packets::ReportPacket;

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 10;

pub const NETWORK_PROTOCOL_VERSION: u16 = 0;

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
        let secret_key = secret_key.unwrap();

        // Prepare packet.
        let packet = ReportPacket {
            reporter_net_protocol: NETWORK_PROTOCOL_VERSION,
            game_report: report,
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
        let mut iv = vec![0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let cipher = Aes256Cbc::new_from_slices(&secret_key, &iv).unwrap();
        let mut encrypted_packet = cipher.encrypt_vec(&binary_packet);

        // Prepare encrypted packet len buffer.
        if encrypted_packet.len() + IV_LENGTH > std::u16::MAX as usize {
            // should never happen
            return (
                ReportResult::InternalError,
                Some(format!(
                    "An error occurred at [{}, {}]: resulting packet is too big ({} > {})",
                    file!(),
                    line!(),
                    encrypted_packet.len() + IV_LENGTH,
                    std::u16::MAX
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
        send_buffer.append(&mut iv);
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

        return (ReportResult::Ok, None);
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

        if secret_key_str.len() < KEY_LENGTH_IN_BYTES {
            if secret_key_str.is_empty() {
                return Err(format!(
                    "An error occurred at [{}, {}]: generated secret key is empty.\n\n",
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
