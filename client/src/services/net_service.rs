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
use crate::misc::app_error::AppError;

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 10;

pub const NETWORK_PROTOCOL_VERSION: u16 = 0;

enum IoResult {
    Ok(usize),
    Fin,
    Err(AppError),
}

pub struct NetService {
    socket: Option<TcpStream>,
    secret_key: Vec<u8>,
}

impl NetService {
    pub fn new() -> Self {
        Self {
            socket: None,
            secret_key: Vec::new(),
        }
    }
    pub fn connect(&mut self, server: String, port: u16, password: String) -> Result<(), AppError> {
        // Connect socket.
        let tcp_socket = TcpStream::connect(format!("{}:{}", server, port));
        if let Err(e) = tcp_socket {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let tcp_socket = tcp_socket.unwrap();

        // Configure socket.
        if let Err(e) = tcp_socket.set_nodelay(true) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        self.socket = Some(tcp_socket);

        // Establish secure connection.
        let secret_key = self.establish_secure_connection();
        if let Err(app_error) = secret_key {
            return Err(app_error.add_entry(file!(), line!()));
        }
        self.secret_key = secret_key.unwrap();

        // Login using password hash.
        // TODO: pass password hash, etc...

        // return control here, don't drop connection, wait for further commands from the user
        Ok(())
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
