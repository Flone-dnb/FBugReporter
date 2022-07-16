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
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_message = String::from(format!(
            "An error occurred: {}\nBacktrace:\n{:?}",
            self.message, self.backtrace
        ));

        write!(f, "{}", error_message)
    }
}
