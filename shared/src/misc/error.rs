// Std.
use std::fmt::Display;

// External.
use backtrace::Backtrace;

#[derive(Debug)]
pub struct AppError {
    message: String,
    backtrace: Backtrace,
}

impl AppError {
    pub fn new(message: &str) -> Self {
        Self {
            message: String::from(message),
            backtrace: Backtrace::new(),
        }
    }
    pub fn get_message(&self) -> String {
        self.message.clone()
    }
    fn backtrace_to_string(&self) -> String {
        let mut current_entry_index: usize = 0;
        let mut output = String::new();

        for frame in self.backtrace.frames() {
            for symbol in frame.symbols() {
                if symbol.filename().is_none() || symbol.lineno().is_none() {
                    continue;
                }
                output += &format!(
                    "{} {}:{}\n",
                    current_entry_index,
                    Self::shorten_backtrace_paths(symbol.filename().unwrap().to_str().unwrap()),
                    symbol.lineno().unwrap()
                );
                current_entry_index += 1;
            }
        }

        output
    }
    fn shorten_backtrace_paths(filename: &str) -> String {
        if filename.contains("rustc") {
            // Probably a crate from standard library.
            return filename.to_string();
        }

        if filename.contains(".cargo") {
            // Probably an external crate.
            return filename[filename.find(".cargo").unwrap()..].to_string();
        }

        let src_dir_pos = filename.find("src");
        if src_dir_pos.is_none() {
            return filename.to_string();
        }
        let src_dir_pos = src_dir_pos.unwrap();

        filename[src_dir_pos..].to_string()
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_message = String::from(format!(
            "An error occurred: {}\nBacktrace:\n{}",
            self.message,
            self.backtrace_to_string()
        ));

        write!(f, "{}", error_message)
    }
}
