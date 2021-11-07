// Std.
use std::env;
use std::io;
use std::io::*;

// Custom.
use services::net_service::NetService;

mod services;

fn main() {
    println!("FBugReporter (server) (v{}).", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' to see commands...\n");

    let net_service = NetService::new();
    if let Err(e) = net_service {
        panic!("{} at [{}, {}]", e, file!(), line!());
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
            println!("config - show the current server configuration");
            println!("refresh-password - generates new server password (use 'config' to display)");
            println!("exit - exit the application");
        } else if input == "start" {
        } else if input == "config" {
            println!("{:#?}", net_service.server_config);
        } else if input == "refresh-password" {
            net_service.refresh_password();
            println!("New password is generated. Please update the password in all client applications in order for them to connect to this server.");
        } else if input == "exit" {
            break;
        } else {
            println!("command '{}' not found", input);
        }

        println!();
    }
}
