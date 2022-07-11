// Std.
use std::env;
use std::io;
use std::io::*;

// Custom.
use services::io::logger_service::Logger;
use services::network::net_service::NetService;

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

    let mut args: Vec<String> = env::args().collect();

    let mut under_monitor = false;
    for arg in args.iter() {
        if arg == "--under-monitor" {
            under_monitor = true;
        }
    }

    if !under_monitor {
        println!();
        println!("---------------------------------------");
        println!("WARNING: you should only run the server using the 'monitor' app");
        println!("WARNING: please, run the 'monitor' app to launch the server");
        println!("---------------------------------------");
        println!();
    }

    loop {
        if let Err(e) = io::stdout().flush() {
            println!("could not flush stdout (error: {}), continuing...", e);
            continue;
        }

        let mut input = String::new();

        if args.len() > 1 {
            if args[1] == "--start" {
                input = "start".to_string();
            }
            args.clear();
        } else {
            if let Err(e) = io::stdin().read_line(&mut input) {
                println!("unable to read input (error: {}), continuing...", e);
                continue;
            }

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
            println!("exit - exit the application");
        } else if input == "start" {
            net_service.start(under_monitor);
        } else if input == "config" {
            println!("{:#?}", net_service.server_config);
        } else if input == "exit" {
            break;
        } else {
            println!("command '{}' not found", input);
        }

        println!();
    }
}
