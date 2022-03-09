// External.
use chrono::prelude::*;
use rand::Rng;
use rusqlite::{params, Connection, Result};
use sha2::{Digest, Sha512};

// Custom.
use crate::{error::AppError, misc::GameReport};

const DATABASE_NAME: &str = "database.db3";
const REPORT_TABLE_NAME: &str = "report";
const USER_TABLE_NAME: &str = "user";

const SALT_LENGTH: u64 = 32;
const PASSWORD_LENGTH: u64 = 32;

// Used for generating random salt, password and etc.
const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                        abcdefghijklmnopqrstuvwxyz\
                        0123456789)(*&^%$#@!~";

pub struct DatabaseManager {
    connection: Connection,
}

impl DatabaseManager {
    pub fn new() -> Result<Self, AppError> {
        let result = Connection::open(DATABASE_NAME);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut connection = result.unwrap();

        // Check 'report' table.
        if let Err(app_error) = DatabaseManager::create_report_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'user' table.
        if let Err(app_error) = DatabaseManager::create_user_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(Self { connection })
    }
    /// Adds a new user to the database.
    ///
    /// On success returns user's password.
    /// On failure returns error description via `AppError`.
    pub fn add_user(&self, username: &str) -> Result<String, AppError> {
        let datetime = Local::now();

        // Generate password and salt.
        let mut rng = rand::thread_rng();
        let plaintext_password: String = (0..PASSWORD_LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        let salt: String = (0..SALT_LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        // Password hash.
        let mut hasher = Sha512::new();
        hasher.update(plaintext_password.as_bytes());
        let mut password_hash = hasher.finalize().to_vec();

        // Salt + 'password hash' hash.
        let mut value: Vec<u8> = Vec::from(salt.as_bytes());
        value.append(&mut password_hash);
        let mut hasher = Sha512::new();
        hasher.update(value.as_slice());
        let password = hasher.finalize().to_vec();

        if let Err(e) = self.connection.execute(
            // password = hash(salt + hash(password))
            &format!(
                "INSERT INTO {} 
            (
                username, 
                salt, 
                password,
                need_change_password,
                last_login_date,
                last_login_time,
                last_login_ip,
                date_registered,
                time_registered
            ) 
            VALUES 
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                USER_TABLE_NAME
            ),
            params![
                username,
                salt,
                password,
                1,
                datetime.date().naive_local().to_string(),
                datetime.time().format("%H:%M:%S").to_string(),
                "",
                datetime.date().naive_local().to_string(),
                datetime.time().format("%H:%M:%S").to_string()
            ],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(plaintext_password)
    }
    /// Removes the user from the database.
    ///
    /// Returns `Ok(true)` if the user was found and removed,
    /// `Ok(false)` if the user was not found.
    /// On failure returns error description via `AppError`.
    pub fn remove_user(&self, username: &str) -> Result<bool, AppError> {
        // See if this user exists.
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT id FROM {} WHERE username='{}'",
                USER_TABLE_NAME, username
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut rows = result.unwrap();

        let row = rows.next();
        if let Err(e) = row {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let row = row.unwrap();
        if row.is_none() {
            return Ok(false);
        }

        // Remove user.
        if let Err(e) = self.connection.execute(
            // password = hash(salt + hash(password))
            &format!(
                "DELETE FROM {}
                WHERE username = '{}'",
                USER_TABLE_NAME, username
            ),
            params![],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(true)
    }
    /// Get password and salt of a user.
    ///
    /// If the user is not found returned `Ok` values will be empty.
    pub fn get_user_password_and_salt(
        &self,
        username: &str,
    ) -> Result<(Vec<u8>, String), AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT password, salt FROM {} WHERE username='{}'",
                USER_TABLE_NAME, username
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut rows = result.unwrap();

        let row = rows.next();
        if let Err(e) = row {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let row = row.unwrap();
        if row.is_none() {
            return Ok((Vec::new(), String::from("")));
        }
        let row = row.unwrap();

        // Get password.
        let password: Result<Vec<u8>> = row.get(0);
        if let Err(e) = password {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let password = password.unwrap();

        // Get salt.
        let salt: Result<String> = row.get(1);
        if let Err(e) = salt {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let salt = salt.unwrap();

        Ok((password, salt))
    }
    /// Updates the following user's values in the database:
    /// - last_login_date
    /// - last_login_time
    /// - last_login_ip
    pub fn update_user_last_login(&self, username: &str, ip: &str) -> Result<(), AppError> {
        let datetime = Local::now();

        if let Err(e) = self.connection.execute(
            &format!(
                "UPDATE {} 
                SET
                last_login_date = ?1,
                last_login_time = ?2,
                last_login_ip = ?3
                WHERE username = ?4",
                USER_TABLE_NAME
            ),
            params![
                datetime.date().naive_local().to_string(),
                datetime.time().format("%H:%M:%S").to_string(),
                ip,
                username
            ],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
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
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    fn create_report_table_if_not_found(connection: &mut Connection) -> Result<(), AppError> {
        // Check if table exists.
        let mut stmt = connection
            .prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                REPORT_TABLE_NAME
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut rows = result.unwrap();
        let row = rows.next().unwrap();

        if row.is_none() {
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
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
        }

        drop(row);
        drop(rows);
        drop(stmt);

        Ok(())
    }
    fn create_user_table_if_not_found(connection: &mut Connection) -> Result<(), AppError> {
        // Check if table exists.
        let mut stmt = connection
            .prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                USER_TABLE_NAME
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut rows = result.unwrap();
        let row = rows.next().unwrap();

        if row.is_none() {
            // Create this table.
            let result = connection.execute(
                // password = hash(salt + hash(password))
                // need_change_password is '1' if the user
                // just registered, thus we need to ask him of a new password.
                &format!(
                    "CREATE TABLE {}(
                    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
                    username             TEXT NOT NULL UNIQUE,
                    salt                 TEXT NOT NULL,
                    password             TEXT NOT NULL,
                    need_change_password INTEGER NOT NULL,
                    last_login_date      TEXT NOT NULL,
                    last_login_time      TEXT NOT NULL,
                    last_login_ip        TEXT NOT NULL,
                    date_registered      TEXT NOT NULL,
                    time_registered      TEXT NOT NULL
                )",
                    USER_TABLE_NAME
                ),
                [],
            );
            if let Err(e) = result {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
        }

        drop(row);
        drop(rows);
        drop(stmt);

        Ok(())
    }
}
