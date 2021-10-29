// Std.
use std::io::prelude::*;
use std::net::*;

// Custom.
use crate::global_params::*;
use crate::logger_service::Logger;
use crate::misc::GameReport;

#[derive(Clone, Copy)]
enum ConnectorAnswer {
    Ok,
    WrongProtocol,
}

pub struct ListenerService {
    connected_socket: Option<TcpStream>,
}

impl ListenerService {
    pub fn new() -> Self {
        Self {
            connected_socket: None,
        }
    }
    pub fn listen_for_report(&mut self, logger: &mut Logger) -> Result<GameReport, ()> {
        // Create socket.
        let listener_socker = TcpListener::bind(format!("127.0.0.1:{}", LISTENER_PORT));
        if let Err(e) = listener_socker {
            logger.log(&format!(
                "An error occurred at [{}, {}]: {:?}",
                file!(),
                line!(),
                e
            ));
            return Err(());
        }
        let listener_socket = listener_socker.unwrap();

        // Wait for connection.
        let accept_result = listener_socket.accept();
        if let Err(e) = accept_result {
            logger.log(&format!(
                "An error occurred at [{}, {}]: {:?}",
                file!(),
                line!(),
                e
            ));
            return Err(());
        }

        let (mut socket, addr) = accept_result.unwrap();
        logger.log(&format!("Accepted connection from port {}.", addr.port()));

        if let Err(e) = socket.set_nodelay(true) {
            logger.log(&format!(
                "An error occurred at [{}, {}]: {:?}",
                file!(),
                line!(),
                e
            ));
            return Err(());
        }

        // Read report.
        let game_report = self.read_report(&mut socket);
        if let Err(msg) = game_report {
            logger.log(&msg);
            return Err(());
        }

        // Save socket.
        self.connected_socket = Some(socket);

        Ok(game_report.unwrap())
    }
    fn read_report(&self, socket: &mut TcpStream) -> Result<GameReport, String> {
        // Read reporter protocol.
        let report_protocol = self.receive_u16(socket);
        if let Err(e) = report_protocol {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let report_protocol = report_protocol.unwrap();

        // Check versions.
        if report_protocol != REPORTER_PROTOCOL {
            // Answer.
            if let Err(e) = self.send_answer(socket, ConnectorAnswer::WrongProtocol) {
                return Err(format!(
                    "An error occurred at [{}, {}], {}",
                    file!(),
                    line!(),
                    e
                ));
            }
        }

        // Read report name.
        let report_name = self.receive_string(socket);
        if let Err(e) = report_name {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let report_name = report_name.unwrap();

        // Read report text.
        let report_text = self.receive_string(socket);
        if let Err(e) = report_text {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let report_text = report_text.unwrap();

        // Read sender name.
        let sender_name = self.receive_string(socket);
        if let Err(e) = sender_name {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let sender_name = sender_name.unwrap();

        // Read sender e-mail.
        let sender_email = self.receive_string(socket);
        if let Err(e) = sender_email {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let sender_email = sender_email.unwrap();

        // Read game name.
        let game_name = self.receive_string(socket);
        if let Err(e) = game_name {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let game_name = game_name.unwrap();

        // Read game version.
        let game_version = self.receive_string(socket);
        if let Err(e) = game_version {
            return Err(format!(
                "An error occurred at [{}, {}], {}",
                file!(),
                line!(),
                e
            ));
        }
        let game_version = game_version.unwrap();

        // Generate OS info.
        let client_os_info = os_info::get();

        // Pack all into struct.
        let game_report = GameReport {
            report_name,
            report_text,
            sender_name,
            sender_email,
            game_name,
            game_version,
            client_os_info,
        };

        Ok(game_report)
    }
    fn receive_string(&self, socket: &mut TcpStream) -> Result<String, String> {
        let mut len_buf = vec![0u8; std::mem::size_of::<u16>()];

        // Read string size.
        match socket.read(&mut len_buf) {
            Ok(0) => {
                return Err(format!(
                    "at [{}, {}]: received unexpected FIN.",
                    file!(),
                    line!()
                ));
            }
            Ok(byte_count) => {
                if byte_count != len_buf.len() {
                    return Err(format!(
                        "at [{}, {}]: received {} bytes of data while expected {}.",
                        file!(),
                        line!(),
                        byte_count,
                        len_buf.len()
                    ));
                }
            }
            Err(e) => {
                return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
            }
        }

        let data_size = bincode::deserialize::<u16>(&len_buf);
        if let Err(e) = data_size {
            return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
        }
        let data_size = data_size.unwrap();

        if data_size == 0 {
            return Ok(String::from("")); // this can happen for e-mail, because it's optional.
        }

        let mut data_buf = vec![0u8; data_size as usize];

        // Read data.
        match socket.read(&mut data_buf) {
            Ok(0) => {
                return Err(format!(
                    "at [{}, {}]: received unexpected FIN.",
                    file!(),
                    line!()
                ));
            }
            Ok(byte_count) => {
                if byte_count != data_buf.len() {
                    return Err(format!(
                        "at [{}, {}]: received {} bytes of data while expected {}.",
                        file!(),
                        line!(),
                        byte_count,
                        data_buf.len()
                    ));
                }
            }
            Err(e) => {
                return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
            }
        }

        let result_data = std::str::from_utf8(&data_buf);
        if let Err(e) = result_data {
            return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
        }

        Ok(String::from(result_data.unwrap()))
    }
    fn send_answer(&self, socket: &mut TcpStream, answer: ConnectorAnswer) -> Result<(), String> {
        let mut _answer_code: u16 = 0;

        match answer {
            // should be just like in the reporter_connector's enum
            ConnectorAnswer::Ok => _answer_code = 0,
            ConnectorAnswer::WrongProtocol => _answer_code = 1,
        }

        let mut answer_buf = bincode::serialize(&_answer_code).unwrap();

        // Send data.
        match socket.write(&mut answer_buf) {
            Ok(0) => {
                return Err(format!(
                    "at [{}, {}]: received unexpected FIN.",
                    file!(),
                    line!()
                ));
            }
            Ok(byte_count) => {
                if byte_count != answer_buf.len() {
                    return Err(format!(
                        "at [{}, {}]: sent {} bytes of data while expected {}.",
                        file!(),
                        line!(),
                        byte_count,
                        answer_buf.len()
                    ));
                }
            }
            Err(e) => {
                return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
            }
        }

        Ok(())
    }
    fn receive_u16(&self, socket: &mut TcpStream) -> Result<u16, String> {
        let mut len_buf = vec![0u8; std::mem::size_of::<u16>()];

        // Read data.
        match socket.read(&mut len_buf) {
            Ok(0) => {
                return Err(format!(
                    "at [{}, {}]: received unexpected FIN.",
                    file!(),
                    line!()
                ));
            }
            Ok(byte_count) => {
                if byte_count != len_buf.len() {
                    return Err(format!(
                        "at [{}, {}]: received {} bytes of data while expected {}.",
                        file!(),
                        line!(),
                        byte_count,
                        len_buf.len()
                    ));
                }
            }
            Err(e) => {
                return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
            }
        }

        let data = bincode::deserialize::<u16>(&len_buf);
        if let Err(e) = data {
            return Err(format!("at [{}, {}]: {:?}", file!(), line!(), e));
        }
        let data = data.unwrap();

        Ok(data)
    }
}
