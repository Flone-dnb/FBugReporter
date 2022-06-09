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
use sha2::{Digest, Sha512};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
const SECRET_KEY_SIZE: usize = 32; // if changed, change protocol version

// Custom.
use super::config_service::ConfigService;
use super::net_packets::*;
use crate::layouts::report_layout::ReportData;
use crate::misc::app_error::AppError;

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 10;

pub const NETWORK_PROTOCOL_VERSION: u16 = 1;

pub enum ConnectResult {
    Connected(bool),
    ConnectFailed(String),
    NeedFirstPassword,
    SetupOTP(String),
    NeedOTP,
    InternalError(AppError),
}

enum IoResult {
    Ok(usize),
    Fin,
    Err(AppError),
}

pub struct NetService {
    socket: Option<TcpStream>,
    secret_key: [u8; SECRET_KEY_SIZE],
    is_connected: bool,
}

impl NetService {
    pub fn new() -> Self {
        Self {
            socket: None,
            secret_key: [0; SECRET_KEY_SIZE],
            is_connected: false,
        }
    }
    /// Tries to connect to the server.
    ///
    /// OTP string might be empty if the user still have not received the OTP QR code.
    /// Once everything is correct, the server will see empty OTP string and if OTP
    /// is enabled for this user (default) the server will respond with OTP QR code
    /// that we can show to the user and connect again with a valid OTP.
    ///
    /// Specify a `new_password` if you want to send the first password (changed password).
    pub fn connect(
        &mut self,
        server: String,
        port: u16,
        username: String,
        password: String,
        otp: String,
        new_password: Option<String>,
    ) -> ConnectResult {
        // Connect socket.
        let tcp_socket = TcpStream::connect(format!("{}:{}", server, port));
        if let Err(e) = tcp_socket {
            return ConnectResult::InternalError(AppError::new(&e.to_string(), file!(), line!()));
        }
        let tcp_socket = tcp_socket.unwrap();

        // Configure socket.
        if let Err(e) = tcp_socket.set_nodelay(true) {
            return ConnectResult::InternalError(AppError::new(&e.to_string(), file!(), line!()));
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return ConnectResult::InternalError(AppError::new(&e.to_string(), file!(), line!()));
        }

        self.socket = Some(tcp_socket);

        // Establish secure connection.
        let secret_key = self.establish_secure_connection();
        if let Err(app_error) = secret_key {
            return ConnectResult::InternalError(app_error.add_entry(file!(), line!()));
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return ConnectResult::InternalError(AppError::new(
                "failed to convert Vec<u8> to generic array",
                file!(),
                line!(),
            ));
        }
        self.secret_key = result.unwrap();

        // Generate password hash.
        let mut hasher = Sha512::new();
        hasher.update(password.as_bytes());
        let password = hasher.finalize().to_vec();

        // Prepare packet to send.
        let mut packet = OutClientPacket::Login {
            client_net_protocol: NETWORK_PROTOCOL_VERSION,
            username: username.clone(),
            password: password.clone(),
            otp,
        };

        if new_password.is_some() {
            // Generate new password hash.
            hasher = Sha512::new();
            hasher.update(new_password.unwrap().as_bytes());
            let new_password = hasher.finalize().to_vec();

            // Update packet to send.
            packet = OutClientPacket::SetFirstPassword {
                client_net_protocol: NETWORK_PROTOCOL_VERSION,
                username: username.clone(),
                old_password: password,
                new_password,
            }
        }

        if let Err(app_error) = self.send_packet(packet) {
            return ConnectResult::InternalError(app_error.add_entry(file!(), line!()));
        }

        // Receive answer.
        let packet = self.receive_packet();
        if let Err(app_error) = packet {
            return ConnectResult::InternalError(app_error.add_entry(file!(), line!()));
        }
        let packet = packet.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<InClientPacket>(&packet);
        if let Err(e) = packet {
            return ConnectResult::InternalError(AppError::new(&e.to_string(), file!(), line!()));
        }
        let packet = packet.unwrap();

        let mut _is_admin = false;
        match packet {
            InClientPacket::LoginAnswer {
                is_ok,
                is_admin,
                fail_reason,
            } => {
                if !is_ok {
                    let mut _message = String::new();
                    match fail_reason.unwrap() {
                        ClientLoginFailReason::WrongProtocol { server_protocol } => {
                            _message = format!(
                                "Failed to connect to the server \
                            due to incompatible application version.\n\
                            Your application uses network protocol version {}, \
                            while the server supports version {}.",
                                NETWORK_PROTOCOL_VERSION, server_protocol
                            );
                        }
                        ClientLoginFailReason::WrongCredentials { result } => match result {
                            ClientLoginFailResult::FailedAttempt {
                                failed_attempts_made,
                                max_failed_attempts,
                            } => {
                                _message = format!(
                                    "Incorrect login/password/OTP.\n\
                                Allowed failed login attempts: {0} out of {1}.\n\
                                After {1} failed login attempts new failed login attempt \
                                 will result in a ban.",
                                    failed_attempts_made, max_failed_attempts
                                );
                            }
                            ClientLoginFailResult::Banned { ban_time_in_min } => {
                                _message = format!(
                                    "You were banned due to multiple failed login attempts.\n\
                                Ban time: {} minute(-s).\n\
                                During this time the server will reject any \
                                login attempts without explanation.",
                                    ban_time_in_min
                                );
                            }
                        },
                        ClientLoginFailReason::SetupOTP { qr_code } => {
                            return ConnectResult::SetupOTP(qr_code);
                        }
                        ClientLoginFailReason::NeedOTP => return ConnectResult::NeedOTP,
                        ClientLoginFailReason::NeedFirstPassword => {
                            return ConnectResult::NeedFirstPassword;
                        }
                    }
                    return ConnectResult::ConnectFailed(_message);
                } else {
                    _is_admin = is_admin;
                }
            }
            _ => {
                return ConnectResult::InternalError(AppError::new(
                    "unexpected packet received",
                    file!(),
                    line!(),
                ));
            }
        }

        // Connected.
        let mut config = ConfigService::new();
        config.server = server;
        config.port = port.to_string();
        config.username = username;
        config.write_config_to_file();

        self.is_connected = true;

        // Return control here, don't drop the connection,
        // wait for further commands from the user.
        ConnectResult::Connected(_is_admin)
    }
    pub fn query_reports(
        &mut self,
        page: u64,
        amount: u64,
    ) -> Result<(Vec<ReportSummary>, u64), AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected", file!(), line!()));
        }

        // Prepare packet to send.
        let packet = OutClientPacket::QueryReportsSummary { page, amount };

        let result = self.send_packet(packet);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        let result = self.receive_packet();
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<InClientPacket>(&serialized_packet);
        if let Err(e) = packet {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let packet = packet.unwrap();

        match packet {
            InClientPacket::ReportsSummary {
                reports,
                total_reports,
            } => {
                return Ok((reports, total_reports));
            }
            _ => {
                return Err(AppError::new(
                    "unexpected packet received",
                    file!(),
                    line!(),
                ));
            }
        }
    }
    pub fn query_report(&mut self, report_id: u64) -> Result<ReportData, AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected", file!(), line!()));
        }

        // Prepare packet to send.
        let packet = OutClientPacket::QueryReport { report_id };

        let result = self.send_packet(packet);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        let result = self.receive_packet();
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<InClientPacket>(&serialized_packet);
        if let Err(e) = packet {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let packet = packet.unwrap();

        match packet {
            InClientPacket::Report {
                id,
                title,
                game_name,
                game_version,
                text,
                date,
                time,
                sender_name,
                sender_email,
                os_info,
            } => {
                return Ok(ReportData {
                    id,
                    title,
                    game_name,
                    game_version,
                    text,
                    date,
                    time,
                    sender_name,
                    sender_email,
                    os_info,
                });
            }
            _ => {
                return Err(AppError::new(
                    "unexpected packet received",
                    file!(),
                    line!(),
                ));
            }
        }
    }
    pub fn delete_report(&mut self, report_id: u64) -> Result<bool, AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected", file!(), line!()));
        }

        // Prepare packet to send.
        let packet = OutClientPacket::DeleteReport { report_id };

        let result = self.send_packet(packet);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        let result = self.receive_packet();
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<InClientPacket>(&serialized_packet);
        if let Err(e) = packet {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let packet = packet.unwrap();

        match packet {
            InClientPacket::DeleteReportResult {
                is_found_and_removed,
            } => {
                return Ok(is_found_and_removed);
            }
            _ => {
                return Err(AppError::new(
                    "unexpected packet received",
                    file!(),
                    line!(),
                ));
            }
        }
    }
    fn receive_packet(&mut self) -> Result<Vec<u8>, AppError> {
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
        match self.read_from_socket(&mut packet_size_buf) {
            IoResult::Fin => {
                return Err(AppError::new(
                    &format!(
                        "unexpected FIN received (socket: {})",
                        match self.socket.as_ref().unwrap().peer_addr() {
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
            IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
            IoResult::Ok(byte_count) => {
                if byte_count != packet_size_buf.len() {
                    return Err(AppError::new(
                        &format!(
                            "not all data received (got: {}, expected: {}) (socket: {})",
                            byte_count,
                            packet_size_buf.len(),
                            match self.socket.as_ref().unwrap().peer_addr() {
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

                let res = bincode::deserialize(&packet_size_buf);
                if let Err(e) = res {
                    return Err(AppError::new(
                        &format!(
                            "{:?} (socket: {})",
                            e,
                            match self.socket.as_ref().unwrap().peer_addr() {
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

                _next_packet_size = res.unwrap();
            }
        }

        // Receive encrypted packet.
        let mut encrypted_packet = vec![0u8; _next_packet_size as usize];
        match self.read_from_socket(&mut encrypted_packet) {
            IoResult::Fin => {
                return Err(AppError::new(
                    &format!(
                        "unexpected FIN received (socket: {})",
                        match self.socket.as_ref().unwrap().peer_addr() {
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
            IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
            IoResult::Ok(_) => {}
        };

        // Get IV.
        if encrypted_packet.len() < IV_LENGTH {
            return Err(AppError::new(
                &format!(
                    "unexpected packet length ({}) (socket: {})",
                    encrypted_packet.len(),
                    match self.socket.as_ref().unwrap().peer_addr() {
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
        let decrypted_packet = Aes256CbcDec::new(&self.secret_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&encrypted_packet);
        if let Err(e) = decrypted_packet {
            return Err(AppError::new(
                &format!(
                    "{:?} (socket: {})",
                    e,
                    match self.socket.as_ref().unwrap().peer_addr() {
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
        let mut decrypted_packet = decrypted_packet.unwrap();

        // CMAC
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
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
                &format!(
                    "{:?} (socket: {})",
                    e,
                    match self.socket.as_ref().unwrap().peer_addr() {
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

        Ok(decrypted_packet)
    }
    fn establish_secure_connection(&mut self) -> Result<Vec<u8>, AppError> {
        // Generate secret key 'b'.
        let mut rng = rand::thread_rng();
        let b: BigUint = rng.sample(RandomBits::new(A_B_BITS));

        // Receive 2 values: p (BigUint), g (BigUint) values.
        // Get 'p' len.
        let mut p_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match self.read_from_socket(&mut p_len_buf) {
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
            match self.read_from_socket(&mut p_buf) {
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
            match self.read_from_socket(&mut g_len_buf) {
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
            match self.read_from_socket(&mut g_buf) {
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
            match self.read_from_socket(&mut a_open_len_buf) {
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
            match self.read_from_socket(&mut a_open_buf) {
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
            match self.write_to_socket(&mut b_open_len_buf) {
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
    fn send_packet(&mut self, packet: OutClientPacket) -> Result<(), AppError> {
        if self.secret_key.is_empty() {
            return Err(AppError::new(
                "can't send packet - secure connected is not established",
                file!(),
                line!(),
            ));
        }

        // Serialize.
        let mut binary_packet = bincode::serialize(&packet).unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
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
        let mut iv = [0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let mut encrypted_packet = Aes256CbcEnc::new(&self.secret_key.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&binary_packet);

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
        send_buffer.append(&mut Vec::from(iv));
        send_buffer.append(&mut encrypted_packet);

        // Send.
        loop {
            match self.write_to_socket(&mut send_buffer) {
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
    fn read_from_socket(&mut self, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
        }

        if self.socket.is_none() {
            return IoResult::Err(AppError::new("the socket is None", file!(), line!()));
        }

        loop {
            match self.socket.as_mut().unwrap().read(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(AppError::new(
                            &format!("failed to read (got: {}, expected: {})", n, buf.len()),
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
                    return IoResult::Err(AppError::new(&e.to_string(), file!(), line!()));
                }
            };
        }
    }
    fn write_to_socket(&mut self, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
        }

        if self.socket.is_none() {
            return IoResult::Err(AppError::new("the socket is None", file!(), line!()));
        }

        loop {
            match self.socket.as_mut().unwrap().write(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(AppError::new(
                            &format!("failed to write (got: {}, expected: {})", n, buf.len()),
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
                    return IoResult::Err(AppError::new(&e.to_string(), file!(), line!()));
                }
            };
        }
    }
}
