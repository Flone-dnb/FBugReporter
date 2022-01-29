// External.
use chrono::prelude::*;
use rusqlite::{params, Connection, Result};

// Custom.
use crate::{error::AppError, misc::GameReport};

const DATABASE_NAME: &str = "database.db3";
const REPORT_TABLE_NAME: &str = "report";

pub struct DatabaseManager {
    connection: Connection,
}

impl DatabaseManager {
    pub fn new() -> Result<Self, String> {
        let result = Connection::open(DATABASE_NAME);
        if let Err(e) = result {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.\n\n",
                file!(),
                line!(),
                e
            ));
        }

        let connection = result.unwrap();

        // Check if table exists.
        let mut stmt = connection
            .prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                REPORT_TABLE_NAME
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?}.\n\n",
                file!(),
                line!(),
                e
            ));
        }

        let mut rows = result.unwrap();
        let row = rows.next().unwrap();

        if row.is_none() {
            println!(
                "INFO: No table \"{}\" was found in database, creating a new table.\n",
                REPORT_TABLE_NAME
            );
            // Create this table.
            let result = connection.execute(
                &format!(
                    "CREATE TABLE {}(
                    id              INTEGER PRIMARY KEY AUTOINCREMENT,
                    report_name     TEXT NOT NULL,
                    report_text     TEXT NOT NULL,
                    sender_name     TEXT NOT NULL,
                    sender_email    TEXT NOT NULL,
                    game_name       TEXT NOT NULL,
                    game_version    TEXT NOT NULL,
                    os_info         TEXT NOT NULL,
                    date_created_at TEXT NOT NULL,
                    time_created_at TEXT NOT NULL     
                )",
                    REPORT_TABLE_NAME
                ),
                [],
            );
            if let Err(e) = result {
                return Err(format!(
                    "An error occurred at [{}, {}]: {:?}.\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }
        }

        drop(row);
        drop(rows);
        drop(stmt);

        Ok(Self { connection })
    }
    pub fn save_report(&self, game_report: GameReport) -> Result<(), AppError> {
        let datetime = Local::now();

        if let Err(e) = self.connection.execute(
            &format!(
                "INSERT INTO {} 
            (
                report_name, 
                report_text, 
                sender_name, 
                sender_email, 
                game_name, 
                game_version, 
                os_info, 
                date_created_at, 
                time_created_at
            ) 
            VALUES 
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                REPORT_TABLE_NAME
            ),
            params![
                game_report.report_name,
                game_report.report_text,
                game_report.sender_name,
                game_report.sender_email,
                game_report.game_name,
                game_report.game_version,
                game_report.client_os_info.to_string(),
                datetime.date().naive_local().to_string(),
                datetime.time().format("%H:%M:%S").to_string()
            ],
        ) {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }

        Ok(())
    }
}
