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
            println!("remove-user <username> - removes a user");
            println!("config - show the current server configuration");
            println!("exit - exit the application");
        } else if input == "start" {
            net_service.start();
        } else if input == "config" {
            println!("{:#?}", net_service.server_config);
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
        } else if input.contains("remove-user ") {
            let username_str: String = input
                .chars()
                .take(0)
                .chain(input.chars().skip("remove-user ".chars().count()))
                .collect();

            if username_str.is_empty() {
                println!("username is empty");
            } else {
                let remove_user_confirm_string = format!("remove user {}", &username_str);
                println!(
                    "Please, confirm the action, type: \"{}\"",
                    remove_user_confirm_string
                );

                io::stdout().flush().ok().expect("could not flush stdout");
                io::stdin()
                    .read_line(&mut input)
                    .expect("unable to read user input");

                input.pop(); // pop '\n'
                if cfg!(windows) {
                    input.pop(); // pop '\r'
                }

                input = String::from(
                    input
                        .strip_prefix(&format!("remove-user {}", &username_str))
                        .unwrap(),
                );

                if input == remove_user_confirm_string {
                    let result = net_service.remove_user(&username_str);
                    if let Err(app_error) = result {
                        panic!("{} at [{}, {}]", app_error.to_string(), file!(), line!());
                    } else {
                        let result = result.unwrap();

                        if result {
                            println!(
                                "The user \"{}\" was removed from the database.",
                                username_str
                            );
                        } else {
                            println!(
                                "A user with the username \"{}\" was not found in the database.",
                                username_str
                            );
                        }
                    }
                } else {
                    println!(
                        "expected: {}\nreceived: {}",
                        remove_user_confirm_string, input
                    );
                }
            }
        } else if input == "exit" {
            break;
        } else {
            println!("command '{}' not found", input);
        }

        println!();
    }
}
