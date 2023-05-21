#![deny(warnings)]

// Std.
use std::env;
use std::io;
use std::io::*;

// Custom.
use shared::misc::db_manager::*;

const ERROR_LOG_PREFIX: &str = "ERROR: ";
const INFO_LOG_PREFIX: &str = "INFO: ";

fn main() {
    println!(
        "FBugReporter (database manager) (v{}).",
        env!("CARGO_PKG_VERSION")
    );
    println!("Type 'help' to see commands...\n");

    let database_location = DatabaseManager::get_database_location();

    println!(
        "{}Looking for database in '{}'...",
        INFO_LOG_PREFIX,
        database_location.to_string_lossy()
    );

    if !(database_location.exists() && database_location.is_file()) {
        println!(
            "{}No database file found at '{}', make sure \
            the database is created (the server will create it once it's started \
            for the first time).",
            ERROR_LOG_PREFIX,
            database_location.to_string_lossy()
        );
        return;
    } else {
        println!(
            "{}Found database at '{}'",
            INFO_LOG_PREFIX,
            database_location.to_string_lossy()
        );
    }

    let database_manager = DatabaseManager::new().unwrap_or_else(|e| panic!("{e}"));

    loop {
        if let Err(e) = io::stdout().flush() {
            println!("could not flush stdout (error: {}), continuing...", e);
            continue;
        }
        let mut input = String::new();

        if let Err(e) = io::stdin().read_line(&mut input) {
            println!("unable to read input (error: {}), continuing...", e);
            continue;
        }

        input.pop(); // pop '\n'
        if cfg!(windows) {
            input.pop(); // pop '\r'
        }

        if input == "help" {
            println!("\ncommands:");
            println!("add-user <username> - adds a new user");
            println!("remove-user <username> - removes a user");
            println!("exit - exit the application");
        } else if input == "exit" {
            break;
        } else if input.contains("add-user ") {
            let username_str: String = input
                .chars()
                .take(0)
                .chain(input.chars().skip("add-user ".chars().count()))
                .collect();

            if username_str.is_empty() {
                println!("username is empty");
            } else {
                println!("should this user be able to delete reports using the client application? (y/n)");
                if let Err(e) = io::stdout().flush() {
                    println!("could not flush stdout (error: {}), continuing...", e);
                    continue;
                }
                if let Err(e) = io::stdin().read_line(&mut input) {
                    println!("unable to read input (error: {}), continuing...", e);
                    continue;
                }

                input.pop(); // pop '\n'
                if cfg!(windows) {
                    input.pop(); // pop '\r'
                }

                input = String::from(
                    input
                        .strip_prefix(&format!("add-user {}", &username_str))
                        .unwrap(),
                );

                if input != "y" && input != "Y" && input != "n" && input != "N" {
                    println!("'{}' is not a valid answer, try again...", input);
                    continue;
                }

                let mut is_admin = false;
                if input == "y" || input == "Y" {
                    is_admin = true;
                }

                let result = database_manager.add_user(&username_str, is_admin);
                match result {
                    AddUserResult::Ok { user_password } => {
                        println!(
                            "New user \"{}\" was registered, user's password is \"{}\".",
                            username_str, user_password
                        );
                    }
                    AddUserResult::NameIsUsed => {
                        println!(
                            "A user with the username \"{}\" already exists in the database.",
                            username_str,
                        );
                    }
                    AddUserResult::NameContainsForbiddenCharacters => {
                        println!(
                            "The username \"{}\" contains forbidden characters, \
                            allowed characters: \"{}\".",
                            username_str, USERNAME_CHARSET
                        );
                    }
                    AddUserResult::Error(e) => {
                        panic!("{} at [{}, {}]", e, file!(), line!());
                    }
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

                if let Err(e) = io::stdout().flush() {
                    println!("could not flush stdout (error: {}), continuing...", e);
                    continue;
                }
                if let Err(e) = io::stdin().read_line(&mut input) {
                    println!("unable to read input (error: {}), continuing...", e);
                    continue;
                }

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
                    let result = database_manager.remove_user(&username_str);
                    if let Err(app_error) = result {
                        panic!("{} at [{}, {}]", app_error, file!(), line!());
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
        } else {
            println!("command '{}' not found", input);
        }

        println!();
    }
}
