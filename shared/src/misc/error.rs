// Std.
use std::fmt::Display;

#[derive(Debug)]
pub struct AppError {
    message: String,
    file: &'static str,
    line: u32,
    stack: Vec<ErrorEntry>,
}

impl AppError {
    pub fn new(message: &str, file: &'static str, line: u32) -> Self {
        Self {
            message: String::from(message),
            file,
            line,
            stack: Vec::new(),
        }
    }
    pub fn add_entry(mut self, file: &'static str, line: u32) -> Self {
        self.stack.push(ErrorEntry { file, line });
        self
    }
    pub fn get_message(&self) -> String {
        self.message.clone()
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut error_message = String::from(format!(
            "An error occurred at [{}, {}]: {}",
            self.file, self.line, self.message,
        ));

        if !self.stack.is_empty() {
            error_message += "\n";
        }

        for error in self.stack.iter() {
            error_message += &format!("- at [{}, {}]\n", error.file, error.line);
        }

        write!(f, "{}", error_message)
    }
}

#[derive(Debug)]
struct ErrorEntry {
    file: &'static str,
    line: u32,
}
