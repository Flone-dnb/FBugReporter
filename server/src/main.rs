// Std.
use std::env;
use std::io;
use std::io::*;

// Custom.
use services::logger_service::Logger;
use services::network::net_service::NetService;

mod error;
mod misc;
mod services;

fn main() {
    println!("FBugReporter (server) (v{}).", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' to see commands...\n");

    let net_service = NetService::new(Logger::new());
    if let Err(err) = net_service {
        let error = err.add_entry(file!(), line!());
        panic!("{}", error);
    }
    let mut net_service = net_service.unwrap();

    let args: Vec<String> = env::args().collect();

    loop {
        io::stdout().flush().ok().expect("could not flush stdout");
        let mut input = String::new();

        if args.len() > 1 {
            if args[1] == "--start" {
                input = "start".to_string();
            }
        } else {
            io::stdin()
                .read_line(&mut input)
                .expect("unable to read user input");

            input.pop(); // pop '\n'
            if cfg!(windows) {
                input.pop(); // pop '\r'
            }
        }

        if input == "help" {
            println!("\noptions:");
            println!("--start - starts the server on launch");
            println!("\ncommands:");
            println!("start - starts the server with the current configuration");
            println!("add-user <username> - adds a new user");
            println!("config - show the current server configuration");
            println!("config.port = <value> - set custom server port value");
            println!("refresh-port - generates new server port");
            println!("exit - exit the application");
        } else if input == "start" {
            net_service.start();
        } else if input.contains("config") {
            if input == "config" {
                println!("{:#?}", net_service.server_config);
            } else if input.contains("config.port = ") {
                let port_str: String = input
                    .chars()
                    .take(0)
                    .chain(input.chars().skip("config.port = ".chars().count()))
                    .collect();

                let port_u16 = port_str.parse::<u16>();
                if let Ok(value) = port_u16 {
                    if let Err(msg) = net_service.set_port(value) {
                        panic!("{} at [{}, {}]", msg, file!(), line!());
                    } else {
                        println!("New port ({}) is saved. Please update the server port in the reporter and the client application in order for them to connect to this server.", value);
                    }
                } else {
                    println!(
                        "can't parse value (maximum value for port is {})",
                        std::u16::MAX
                    );
                }
            } else {
                println!("command '{}' not found", input);
            }
        } else if input.contains("add-user ") {
            let username_str: String = input
                .chars()
                .take(0)
                .chain(input.chars().skip("add-user ".chars().count()))
                .collect();

            if username_str.is_empty() {
                println!("username is empty");
            } else {
                let result = net_service.add_user(&username_str);
                if let Err(app_error) = result {
                    panic!("{} at [{}, {}]", app_error.to_string(), file!(), line!());
                } else {
                    println!(
                        "New user \"{}\" was registered, user's password is \"{}\".",
                        username_str,
                        result.unwrap()
                    );
                }
            }
        } else if input == "refresh-port" {
            if let Err(msg) = net_service.refresh_port() {
                panic!("{} at [{}, {}]", msg, file!(), line!());
            }
            println!("New port is generated. Please update the server port in all client applications in order for them to connect to this server.");
        } else if input == "exit" {
            break;
        } else {
            println!("command '{}' not found", input);
        }

        println!();
    }
}
