#![deny(warnings)]

// Std.
use std::env;
use std::fs;
use std::process::Command;

/// Looks if there is a server file in the current directory.
/// Returns `true` if a server was found, `false` if not.
fn find_server(server: &str) -> bool {
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

        if path.file_type().unwrap().is_file() && path.file_name().to_str().unwrap() == server {
            return true;
        }
    }

    false
}

fn main() {
    let mut server_binary_name = String::from("server");

    if cfg!(windows) {
        server_binary_name += ".exe";
    }

    if !find_server(&server_binary_name) {
        println!(
            "No '{}' file was found in the current directory, make sure \
            the server executable exists.",
            &server_binary_name
        );
        return;
    }

    let path = env::current_dir().unwrap().join(server_binary_name);

    loop {
        let mut process = match Command::new(path.clone())
            .args(["--start", "--under-monitor"])
            .spawn()
        {
            Ok(process) => process,
            Err(err) => panic!("Failed to start the server, error: {}", err),
        };

        println!("Started a new server process. The server is running.");

        let result = process.wait();
        if let Err(ref e) = result {
            println!("Failed to wait for server process, error: {}", e);
        }

        println!("The server process exited with code {}\n", result.unwrap());
    }
}
