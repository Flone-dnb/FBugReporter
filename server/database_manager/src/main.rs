// Std.
use std::env;
use std::fs;
use std::io;
use std::io::*;

// Custom.
use shared::misc::db_manager::*;

/// Looks if there is a database file in the current directory.
/// Returns `true` if a database was found, `false` if not.
fn find_database() -> bool {
    let paths = fs::read_dir(".");
    if let Err(e) = paths {
        panic!("{}", e);
    }
    let paths = paths.unwrap();

    for path in paths {
        if let Err(e) = path {
            panic!("{}", e);
        }
        let path = path.unwrap();

        if path.file_type().unwrap().is_file() {
            if path.file_name().to_str().unwrap() == DATABASE_NAME {
                return true;
            }
        }
    }

    false
}

fn main() {
    println!(
        "FBugReporter (database manager) (v{}).",
        env!("CARGO_PKG_VERSION")
    );
    println!("Type 'help' to see commands...\n");

    if !find_database() {
        println!(
            "No '{}' file was found in the current directory, make sure \
            the database is created (the server will create it once it's started \
            for the first time).",
            DATABASE_NAME
        );
        return;
    }

    let database_manager = DatabaseManager::new();
    if let Err(app_error) = database_manager {
        let app_error = app_error.add_entry(file!(), line!());
        panic!("{}", app_error);
    }
    let database_manager = database_manager.unwrap();

    loop {
        if let Err(e) = io::stdout().flush() {
            println!(
                "could not flush stdout (error: {}), continuing...",
                e.to_string()
            );
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
                    println!(
                        "could not flush stdout (error: {}), continuing...",
                        e.to_string()
                    );
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
                        panic!("{} at [{}, {}]", e.to_string(), file!(), line!());
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
                    println!(
                        "could not flush stdout (error: {}), continuing...",
                        e.to_string()
                    );
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
        } else {
            println!("command '{}' not found", input);
        }

        println!();
    }
}
