use std::char::REPLACEMENT_CHARACTER;

use rusqlite::{params, Connection, Result};

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
    pub fn save_report() -> Result<(), String> {
        Ok(())
    }
}
