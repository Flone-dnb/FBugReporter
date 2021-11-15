mod listener_service;
mod logger_service;
mod misc;
mod reporter_service;
use listener_service::*;
use logger_service::*;
use misc::GameReport;
use reporter_service::*;

fn main() {
    // Prepare logging.
    let mut logger = Logger::new();
    logger.log("Starting.");

    // Wait for report.
    let mut listener = ListenerService::new();
    logger.log("Listening for report...");
    let game_report = listener.listen_for_report(&mut logger);
    if game_report.is_err() {
        return;
    }
    let (game_report, server_addr) = game_report.unwrap();

    logger.log("Received a report.");

    // Send report.
    // TODO
    println!("{:?}", game_report);
}
