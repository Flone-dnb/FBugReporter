// Std.
use std::net::*;

// Custom.
use crate::logger_service::Logger;

pub const REPORTER_PROTOCOL_VERSION: u16 = 0;

pub struct ReporterService {}

impl ReporterService {
    pub fn new() -> Self {
        Self {}
    }
    pub fn send_report(logger: &mut Logger) {}
}
