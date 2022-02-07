// Std.
use std::net::TcpStream;

// Custom.
use crate::misc::app_error::AppError;

pub struct NetService {
    socket: Option<TcpStream>,
}

impl NetService {
    pub fn new() -> Self {
        Self { socket: None }
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

        // Establish secure connection.
        // TODO: establish secure connection.

        // Login using password hash.
        // TODO: pass password hash, etc...

        // return control here, don't drop connection, wait for further commands from the user
        Ok(())
    }
}
