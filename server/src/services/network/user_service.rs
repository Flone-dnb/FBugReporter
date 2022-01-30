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
use rand::{Rng, RngCore};

// Custom.
use crate::error::AppError;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;
const KEY_LENGTH_IN_BYTES: usize = 32; // if changed, change protocol version

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const NETWORK_PROTOCOL_VERSION: u16 = 0;
const MAX_PACKET_SIZE_IN_BYTES: u32 = 131_072; // 128 kB for now
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 10;

// Custom.
use super::net_packets::{InPacket, OutPacket};
use crate::misc::*;
use crate::services::db_manager::DatabaseManager;
use crate::services::logger_service::Logger;

enum IoResult {
    Ok(usize),
    Fin,
    Err(AppError),
}
pub struct UserService {
    logger: Arc<Mutex<Logger>>,
    socket: TcpStream,
    secret_key: Vec<u8>,
    connected_users_count: Arc<Mutex<usize>>,
    exit_error: Option<(ReportResult, AppError)>,
    database: Arc<Mutex<DatabaseManager>>,
}

impl UserService {
    pub fn new(
        logger: Arc<Mutex<Logger>>,
        socket: TcpStream,
        addr: SocketAddr,
        connected_users_count: Arc<Mutex<usize>>,
        database: Arc<Mutex<DatabaseManager>>,
    ) -> Self {
        {
            let mut guard = connected_users_count.lock().unwrap();
            *guard += 1;
            logger.lock().unwrap().print_and_log(&format!(
                "Accepted connection with {}:{}\n--- [connected: {}]",
                addr.ip(),
                addr.port(),
                guard
            ));
        }

        UserService {
            logger,
            socket,
            connected_users_count,
            exit_error: None,
            secret_key: Vec::new(),
            database,
        }
    }
    /// After this function is finished the object is destroyed.
    pub fn process_user(&mut self) {
        let secret_key = UserService::establish_secure_connection(&mut self.socket);
        if let Err(err) = secret_key {
            self.exit_error = Some((ReportResult::InternalError, err.add_entry(file!(), line!())));
            return;
        }
        self.secret_key = secret_key.unwrap();

        let packet = self.receive_packet();
        if let Err(err) = packet {
            self.exit_error = Some((ReportResult::InternalError, err.add_entry(file!(), line!())));
            return;
        }
        let packet = packet.unwrap();

        if let Err(result) = self.handle_packet(packet) {
            self.exit_error = Some((result.0, result.1.add_entry(file!(), line!())));
            return;
        }
    }
    fn handle_packet(&mut self, packet: InPacket) -> Result<(), (ReportResult, AppError)> {
        // TODO: check protocol version (send ReportResult::WrongProtocol if different)
        // TODO: in wouldblock (read/write functions) have loop limit as a variable
        //       never set the limit when waiting for user messages!
        //       pass loop limit to read/write functions
        // TODO: add keep alive timer (for clients only)
        // TODO: (only for clients, not for reporters) check password hash and etc (send ReportResult::NetworkIssue if cmac or other errors)
        match packet {
            InPacket::ReportPacket {
                reporter_net_protocol,
                game_report,
            } => {
                if let Err(err) = self.handle_reporter(reporter_net_protocol, game_report) {
                    return Err((err.0, err.1.add_entry(file!(), line!())));
                }
            }
        }

        Ok(())
    }
    fn handle_reporter(
        &mut self,
        reporter_net_protocol: u16,
        game_report: GameReport,
    ) -> Result<(), (ReportResult, AppError)> {
        // Check protocol version.
        if reporter_net_protocol != NETWORK_PROTOCOL_VERSION {
            let result_code = ReportResult::WrongProtocol;

            if let Err(err) = UserService::send_packet(
                &mut self.socket,
                &self.secret_key,
                OutPacket::ReportAnswer { result_code },
            ) {
                return Err((ReportResult::InternalError, err.add_entry(file!(), line!())));
            }

            return Err((
                result_code,
                AppError::new(
                    &format!(
                        "wrong protocol version ({} != {}) (socket: {})",
                        reporter_net_protocol,
                        NETWORK_PROTOCOL_VERSION,
                        self.socket.peer_addr().unwrap()
                    ),
                    file!(),
                    line!(),
                ),
            ));
        }

        // Check field limits.
        if let Err((field, length)) = UserService::check_report_field_limits(&game_report) {
            let result_code = ReportResult::ServerRejected;

            if let Err(err) = UserService::send_packet(
                &mut self.socket,
                &self.secret_key,
                OutPacket::ReportAnswer { result_code },
            ) {
                return Err((ReportResult::InternalError, err.add_entry(file!(), line!())));
            }

            return Err((
                result_code,
                AppError::new(
            &format!(
                        "report exceeds report field limits ({:?} has length of {} characters while the limit is {}) (socket: {})",
                        field,
                        length,
                        field.max_length(),
                        self.socket.peer_addr().unwrap()
                    ),
               file!(),
               line!(),
                ),
            ));
        }

        self.logger.lock().unwrap().print_and_log(&format!(
            "Received a report from socket {}",
            self.socket.peer_addr().unwrap()
        ));

        {
            if let Err(err) = self.database.lock().unwrap().save_report(game_report) {
                let result_code = ReportResult::InternalError;

                if let Err(err) = UserService::send_packet(
                    &mut self.socket,
                    &self.secret_key,
                    OutPacket::ReportAnswer { result_code },
                ) {
                    return Err((ReportResult::InternalError, err.add_entry(file!(), line!())));
                }

                return Err((result_code, err.add_entry(file!(), line!())));
            }
        }

        self.logger.lock().unwrap().print_and_log(&format!(
            "Saved a report from socket {}",
            self.socket.peer_addr().unwrap()
        ));

        // Answer "OK".
        if let Err(err) = UserService::send_packet(
            &mut self.socket,
            &self.secret_key,
            OutPacket::ReportAnswer {
                result_code: ReportResult::Ok,
            },
        ) {
            return Err((ReportResult::InternalError, err.add_entry(file!(), line!())));
        }

        Ok(())
    }
    fn establish_secure_connection(socket: &mut TcpStream) -> Result<Vec<u8>, AppError> {
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
        loop {
            match UserService::write_to_socket(socket, &mut pg_send_buf) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
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
        loop {
            match UserService::write_to_socket(socket, &mut a_open_len_buf) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        // Receive open key 'B' size.
        let mut b_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match UserService::read_from_socket(socket, &mut b_open_len_buf) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        // Receive open key 'B'.
        let b_open_len = bincode::deserialize::<u64>(&b_open_len_buf);
        if let Err(e) = b_open_len {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let b_open_len = b_open_len.unwrap();
        let mut b_open_buf = vec![0u8; b_open_len as usize];

        loop {
            match UserService::read_from_socket(socket, &mut b_open_buf) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let b_open_big = bincode::deserialize::<BigUint>(&b_open_buf);
        if let Err(e) = b_open_big {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let b_open_big = b_open_big.unwrap();

        // Calculate the secret key.
        let secret_key = b_open_big.modpow(&a, &p);
        let mut secret_key_str = secret_key.to_str_radix(10);

        if secret_key_str.len() < KEY_LENGTH_IN_BYTES {
            if secret_key_str.is_empty() {
                return Err(AppError::new(
                    "generated secret key is empty",
                    file!(),
                    line!(),
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
    /// Returns [`Ok`] if the fields have the corrent length (amount of characters, not byte count),
    /// otherwise returns the field type and its received length (not the limit, actual length).
    fn check_report_field_limits(report: &GameReport) -> Result<(), (ReportLimits, usize)> {
        if report.report_name.chars().count() > ReportLimits::ReportName.max_length() {
            return Err((ReportLimits::ReportName, report.report_name.chars().count()));
        }

        if report.report_text.chars().count() > ReportLimits::ReportText.max_length() {
            return Err((ReportLimits::ReportText, report.report_text.chars().count()));
        }

        if report.sender_name.chars().count() > ReportLimits::SenderName.max_length() {
            return Err((ReportLimits::SenderName, report.sender_name.chars().count()));
        }

        if report.sender_email.chars().count() > ReportLimits::SenderEMail.max_length() {
            return Err((
                ReportLimits::SenderEMail,
                report.sender_email.chars().count(),
            ));
        }

        if report.game_name.chars().count() > ReportLimits::GameName.max_length() {
            return Err((ReportLimits::GameName, report.game_name.chars().count()));
        }

        if report.game_version.chars().count() > ReportLimits::GameVersion.max_length() {
            return Err((
                ReportLimits::GameVersion,
                report.game_version.chars().count(),
            ));
        }

        Ok(())
    }
    fn send_packet(
        socket: &mut TcpStream,
        secret_key: &[u8],
        packet: OutPacket,
    ) -> Result<(), AppError> {
        if secret_key.is_empty() {
            return Err(AppError::new(
                "can't send packet - secure connected is not established",
                file!(),
                line!(),
            ));
        }

        // Serialize.
        let mut binary_packet = bincode::serialize(&packet).unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&secret_key).unwrap();
        mac.update(&binary_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return Err(AppError::new(
                &format!(
                    "unexpected tag length: {} != {}",
                    tag_bytes.len(),
                    CMAC_TAG_LENGTH
                ),
                file!(),
                line!(),
            ));
        }

        binary_packet.append(&mut tag_bytes);

        // Encrypt packet.
        let mut rng = rand::thread_rng();
        let mut iv = vec![0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let cipher = Aes256Cbc::new_from_slices(&secret_key, &iv).unwrap();
        let mut encrypted_packet = cipher.encrypt_vec(&binary_packet);

        // Prepare encrypted packet len buffer.
        if encrypted_packet.len() + IV_LENGTH > std::u32::MAX as usize {
            // should never happen
            return Err(AppError::new(
                &format!(
                    "resulting packet is too big ({} > {})",
                    encrypted_packet.len() + IV_LENGTH,
                    std::u32::MAX
                ),
                file!(),
                line!(),
            ));
        }
        let encrypted_len = (encrypted_packet.len() + IV_LENGTH) as u32;
        let encrypted_len_buf = bincode::serialize(&encrypted_len);
        if let Err(e) = encrypted_len_buf {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let mut send_buffer = encrypted_len_buf.unwrap();

        // Merge all to one buffer.
        send_buffer.append(&mut iv);
        send_buffer.append(&mut encrypted_packet);

        // Send to the server.
        loop {
            match UserService::write_to_socket(socket, &mut send_buffer) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        Ok(())
    }
    fn receive_packet(&mut self) -> Result<InPacket, AppError> {
        if self.secret_key.is_empty() {
            return Err(AppError::new(
                "can't receive packet - secure connected is not established",
                file!(),
                line!(),
            ));
        }

        // Read u32 (size of a packet)
        let mut packet_size_buf = [0u8; std::mem::size_of::<u32>() as usize];
        let mut _next_packet_size: u32 = 0;
        match UserService::read_from_socket(&mut self.socket, &mut packet_size_buf) {
            IoResult::Fin => {
                return Err(AppError::new(
                    &format!(
                        "unexpected FIN received (socket: {})",
                        self.socket.peer_addr().unwrap()
                    ),
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
                            self.socket.peer_addr().unwrap()
                        ),
                        file!(),
                        line!(),
                    ));
                }

                let res = bincode::deserialize(&packet_size_buf);
                if let Err(e) = res {
                    return Err(AppError::new(
                        &format!("{:?} (socket: {})", e, self.socket.peer_addr().unwrap()),
                        file!(),
                        line!(),
                    ));
                }

                _next_packet_size = res.unwrap();
            }
        }

        // Check packet size.
        if _next_packet_size > MAX_PACKET_SIZE_IN_BYTES {
            return Err(AppError::new(
                &format!(
                    "incoming packet is too big to receive ({} > {} bytes) (socket: {})",
                    _next_packet_size,
                    MAX_PACKET_SIZE_IN_BYTES,
                    self.socket.peer_addr().unwrap()
                ),
                file!(),
                line!(),
            ));
        }

        // Receive encrypted packet.
        let mut encrypted_packet = vec![0u8; _next_packet_size as usize];
        match UserService::read_from_socket(&mut self.socket, &mut encrypted_packet) {
            IoResult::Fin => {
                return Err(AppError::new(
                    &format!(
                        "unexpected FIN received (socket: {})",
                        self.socket.peer_addr().unwrap()
                    ),
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
                    self.socket.peer_addr().unwrap()
                ),
                file!(),
                line!(),
            ));
        }
        let iv = &encrypted_packet[..IV_LENGTH].to_vec();
        encrypted_packet = encrypted_packet[IV_LENGTH..].to_vec();

        // Decrypt packet.
        let cipher = Aes256Cbc::new_from_slices(&self.secret_key, &iv).unwrap();
        let decrypted_packet = cipher.decrypt_vec(&encrypted_packet);
        if let Err(e) = decrypted_packet {
            return Err(AppError::new(
                &format!("{:?} (socket: {})", e, self.socket.peer_addr().unwrap()),
                file!(),
                line!(),
            ));
        }
        let mut decrypted_packet = decrypted_packet.unwrap();

        // CMAC
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        let tag: Vec<u8> = decrypted_packet
            .drain(decrypted_packet.len().saturating_sub(CMAC_TAG_LENGTH)..)
            .collect();
        mac.update(&decrypted_packet);
        if let Err(e) = mac.verify(&tag) {
            return Err(AppError::new(
                &format!("{:?} (socket: {})", e, self.socket.peer_addr().unwrap()),
                file!(),
                line!(),
            ));
        }

        // Deserialize.
        let packet = bincode::deserialize::<InPacket>(&decrypted_packet);
        if let Err(e) = packet {
            return Err(AppError::new(
                &format!("{:?} (socket: {})", e, self.socket.peer_addr().unwrap()),
                file!(),
                line!(),
            ));
        }

        Ok(packet.unwrap())
    }
    fn read_from_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
        }

        loop {
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
                                socket.peer_addr().unwrap(),
                            ),
                            file!(),
                            line!(),
                        ));
                    }

                    return IoResult::Ok(n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    continue;
                }
                Err(e) => {
                    return IoResult::Err(AppError::new(
                        &format!("{:?} (socket {})", e, socket.peer_addr().unwrap(),),
                        file!(),
                        line!(),
                    ));
                }
            };
        }
    }
    fn write_to_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
        }

        loop {
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
                                socket.peer_addr().unwrap(),
                            ),
                            file!(),
                            line!(),
                        ));
                    }

                    return IoResult::Ok(n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    continue;
                }
                Err(e) => {
                    return IoResult::Err(AppError::new(
                        &format!("{:?} (socket {})", e, socket.peer_addr().unwrap()),
                        file!(),
                        line!(),
                    ));
                }
            };
        }
    }
}

impl Drop for UserService {
    fn drop(&mut self) {
        let mut message = format!(
            "Closing connection with {}",
            self.socket.peer_addr().unwrap()
        );

        if self.exit_error.is_some() {
            message += " due to error:\n";
            message += &format!("({:?}): ", self.exit_error.as_ref().unwrap().0);
            message += &self.exit_error.as_ref().unwrap().1.to_string();
        }

        message += "\n";

        let mut guard = self.connected_users_count.lock().unwrap();
        *guard -= 1;
        message += &format!("--- [connected: {}]", guard);

        self.logger.lock().unwrap().print_and_log(&message);
    }
}
