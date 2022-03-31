// External.
use chrono::prelude::*;
use rand::Rng;
use rusqlite::{params, Connection, Result};
use sha2::{Digest, Sha512};

// Custom.
use super::network::net_packets::ReportSummary;
use crate::{error::AppError, misc::GameReport};

const DATABASE_NAME: &str = "database.db3";

const REPORT_TABLE_NAME: &str = "report";
const USER_TABLE_NAME: &str = "user";
const VERSION_TABLE_NAME: &str = "version";

const REPORT_TABLE_HASH: &[u8] = &[
    61, 115, 240, 65, 82, 203, 101, 187, 246, 7, 57, 65, 37, 33, 146, 7, 116, 211, 101, 99, 145,
    128, 108, 127, 232, 41, 47, 6, 53, 22, 244, 58, 186, 221, 227, 37, 199, 217, 134, 49, 226, 79,
    164, 250, 215, 136, 114, 72, 71, 180, 83, 50, 211, 251, 214, 2, 127, 131, 105, 113, 23, 43, 25,
    226,
];
const USER_TABLE_HASH: &[u8] = &[
    179, 186, 80, 246, 153, 112, 181, 236, 128, 174, 246, 56, 18, 207, 187, 205, 195, 233, 114,
    232, 101, 232, 167, 62, 117, 146, 20, 245, 92, 174, 204, 9, 51, 25, 167, 63, 70, 24, 23, 20,
    16, 224, 225, 200, 12, 51, 191, 225, 22, 39, 142, 125, 162, 199, 74, 76, 200, 218, 189, 66,
    243, 47, 180, 120,
];
const VERSION_TABLE_HASH: &[u8] = &[
    94, 44, 135, 144, 199, 240, 160, 192, 134, 20, 58, 5, 166, 161, 247, 8, 243, 133, 150, 243,
    170, 153, 111, 171, 177, 44, 21, 21, 216, 125, 205, 17, 4, 246, 33, 52, 25, 191, 115, 13, 109,
    97, 3, 238, 17, 191, 125, 170, 23, 161, 189, 182, 35, 114, 128, 169, 94, 60, 56, 147, 84, 54,
    43, 145,
];

const SUPPORTED_DATABASE_VERSION: u64 = 0;

const SALT_LENGTH: u64 = 32;
const OTP_SECRET_LENGTH: u64 = 256;
const PASSWORD_LENGTH: u64 = 32;

// Used for generating random salt, password, otp and etc.
const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                        abcdefghijklmnopqrstuvwxyz\
                        0123456789)(*&^%$#@!~";

pub const USERNAME_CHARSET: &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                        abcdefghijklmnopqrstuvwxyz\
                        0123456789.";

pub enum AddUserResult {
    Ok { user_password: String },
    NameIsUsed,
    NameContainsForbiddenCharacters,
    Error(AppError),
}

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

        // Check 'version' table.
        if let Err(app_error) = DatabaseManager::create_version_table_if_not_found(&mut connection)
        {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'report' table.
        if let Err(app_error) = DatabaseManager::create_report_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'user' table.
        if let Err(app_error) = DatabaseManager::create_user_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        if let Err(app_error) = DatabaseManager::handle_old_database_version(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(Self { connection })
    }
    pub fn get_reports(&self, mut page: u64, amount: u64) -> Result<Vec<ReportSummary>, AppError> {
        if page == 0 {
            page = 1;
        }

        let start_id: u64 = (page - 1) * amount;

        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT id, report_name, game_name, date_created_at, time_created_at \
                 FROM {} WHERE id > {} \
                 ORDER BY id LIMIT {}",
                REPORT_TABLE_NAME, start_id, amount
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut rows = result.unwrap();

        let mut reports: Vec<ReportSummary> = Vec::new();

        loop {
            let row = rows.next();
            if let Err(e) = row {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let row = row.unwrap();
            if row.is_none() {
                return Ok(reports);
            }

            let row = row.unwrap();

            // Get report id.
            let id = row.get(0);
            if let Err(e) = id {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let id: u64 = id.unwrap();

            // Get report title.
            let title = row.get(1);
            if let Err(e) = title {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let title: String = title.unwrap();

            // Get report game name.
            let game = row.get(2);
            if let Err(e) = game {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let game: String = game.unwrap();

            // Get report date.
            let date = row.get(3);
            if let Err(e) = date {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let date: String = date.unwrap();

            // Get report date.
            let time = row.get(4);
            if let Err(e) = time {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let time: String = time.unwrap();

            reports.push(ReportSummary {
                id,
                title,
                game,
                date,
                time,
            })
        }
    }
    /// Adds a new user to the database.
    pub fn add_user(&self, username: &str) -> AddUserResult {
        // Check if username contains forbidden characters.
        let is_ok = username
            .chars()
            .all(|c| USERNAME_CHARSET.chars().any(|allowed| c == allowed));
        if !is_ok {
            return AddUserResult::NameContainsForbiddenCharacters;
        }

        // Check if username is used.
        let result = self.is_user_exists(username);
        if let Err(e) = result {
            return AddUserResult::Error(AppError::new(&e.to_string(), file!(), line!()));
        }
        let exists = result.unwrap();
        if exists {
            return AddUserResult::NameIsUsed;
        }

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

        // OTP secret.
        let otp_secret: String = (0..OTP_SECRET_LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        if let Err(e) = self.connection.execute(
            // password = hash(salt + hash(password))
            &format!(
                "INSERT INTO {} 
            (
                username, 
                salt, 
                password,
                need_change_password,
                need_setup_otp,
                otp_secret_key,
                last_login_date,
                last_login_time,
                last_login_ip,
                date_registered,
                time_registered
            ) 
            VALUES 
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                USER_TABLE_NAME
            ),
            params![
                username,
                salt,
                password,
                1, // change password
                1, // have not received OTP QR code
                otp_secret,
                datetime.date().naive_local().to_string(),
                datetime.time().format("%H:%M:%S").to_string(),
                "",
                datetime.date().naive_local().to_string(),
                datetime.time().format("%H:%M:%S").to_string()
            ],
        ) {
            return AddUserResult::Error(AppError::new(&e.to_string(), file!(), line!()));
        }

        AddUserResult::Ok {
            user_password: plaintext_password,
        }
    }
    /// Removes the user from the database.
    ///
    /// Returns `Ok(true)` if the user was found and removed,
    /// `Ok(false)` if the user was not found.
    /// On failure returns error description via `AppError`.
    pub fn remove_user(&self, username: &str) -> Result<bool, AppError> {
        let result = self.is_user_exists(username);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let exists = result.unwrap();
        if exists == false {
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
    /// Check if a given user needs to change the password.
    ///
    /// Returns `Ok(true)` if need to change the password, `Ok(false)` if not.
    /// On failure returns `AppError`.
    pub fn is_user_needs_to_change_password(&self, username: &str) -> Result<bool, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT need_change_password FROM {} WHERE username='{}'",
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
            return Err(AppError::new(
                &format!("database returned None for username {}", username),
                file!(),
                line!(),
            ));
        }
        let row = row.unwrap();
        let need_change_password = row.get(0);
        if let Err(e) = need_change_password {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let need_change_password: i32 = need_change_password.unwrap();

        if need_change_password == 1 {
            return Ok(true);
        } else if need_change_password == 0 {
            return Ok(false);
        } else {
            return Err(AppError::new(
                &format!(
                    "database returned 'need_change_password' equal to '{}' for user {}",
                    need_change_password, username
                ),
                file!(),
                line!(),
            ));
        }
    }
    /// Check if a given user needs to setup OTP (receive OTP QR code).
    ///
    /// Returns `Ok(true)` if need OTP QR code, `Ok(false)` if not.
    /// On failure returns `AppError`.
    pub fn is_user_needs_setup_otp(&self, username: &str) -> Result<bool, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT need_setup_otp FROM {} WHERE username='{}'",
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
            return Err(AppError::new(
                &format!("database returned None for username {}", username),
                file!(),
                line!(),
            ));
        }
        let row = row.unwrap();
        let need_setup_otp = row.get(0);
        if let Err(e) = need_setup_otp {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let need_setup_otp: i32 = need_setup_otp.unwrap();

        if need_setup_otp == 1 {
            return Ok(true);
        } else if need_setup_otp == 0 {
            return Ok(false);
        } else {
            return Err(AppError::new(
                &format!(
                    "database returned 'need_setup_otp' equal to '{}' for user {}",
                    need_setup_otp, username
                ),
                file!(),
                line!(),
            ));
        }
    }
    /// Returns OTP secret key.
    pub fn get_otp_secret_key_for_user(&self, username: &str) -> Result<String, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT otp_secret_key FROM {} WHERE username='{}'",
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
            return Err(AppError::new(
                &format!("database returned None for username {}", username),
                file!(),
                line!(),
            ));
        }
        let row = row.unwrap();
        let otp_secret = row.get::<usize, String>(0);
        if let Err(e) = otp_secret {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let otp_secret = otp_secret.unwrap();

        return Ok(otp_secret);
    }
    /// Sets new password for user.
    ///
    /// Returns `Ok(true)` if user don't need to change password (error), `Ok(false)`
    /// if changed successfully.
    ///
    /// On failure returns `AppError`.
    pub fn update_user_password(
        &self,
        username: &str,
        mut new_password: Vec<u8>,
    ) -> Result<bool, AppError> {
        // Check if user needs to change his password.
        let result = self.is_user_needs_to_change_password(username);
        if let Err(e) = result {
            return Err(e.add_entry(file!(), line!()));
        }
        let need_change_password = result.unwrap();
        if need_change_password == false {
            return Ok(true);
        }

        let result = self.get_user_password_and_salt(username);
        if let Err(e) = result {
            return Err(e.add_entry(file!(), line!()));
        }
        let (_current_password, salt) = result.unwrap();

        // Salt + 'password hash' hash.
        let mut value: Vec<u8> = Vec::from(salt.as_bytes());
        value.append(&mut new_password);
        let mut hasher = Sha512::new();
        hasher.update(value.as_slice());
        let password = hasher.finalize().to_vec();

        // Update password and 'need_to_change_password' in database.
        let result = self.connection.execute(
            &format!(
                "UPDATE {} SET password = ?1, need_change_password = 0 WHERE username = '{}'",
                USER_TABLE_NAME, username
            ),
            [password],
        );
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(false)
    }
    /// Marks that user have setup OTP and no longer needs OTP QR code.
    pub fn set_user_finished_otp_setup(&self, username: &str) -> Result<(), AppError> {
        // Check if user needs to setup OTP.
        let result = self.is_user_needs_setup_otp(username);
        if let Err(e) = result {
            return Err(e.add_entry(file!(), line!()));
        }
        let need_change_password = result.unwrap();
        if need_change_password == false {
            return Err(AppError::new(
                &format!(
                    "user \"{}\" already setup OTP \
                    but we requested to finish OTP setup",
                    username
                ),
                file!(),
                line!(),
            ));
        }

        // Update 'need_setup_otp' in database.
        let result = self.connection.execute(
            &format!(
                "UPDATE {} SET need_setup_otp = 0 WHERE username = '{}'",
                USER_TABLE_NAME, username
            ),
            [],
        );
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    /// Check if a given user exists in the database.
    ///
    /// Returns `Ok(true)` if the user exists, `Ok(false)` if not.
    /// On failure returns `AppError`.
    fn is_user_exists(&self, username: &str) -> Result<bool, AppError> {
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

        Ok(true)
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
            // TODO: IF CHANGING TABLE STRUCTURE,
            // TODO: remember to increment SUPPORTED_DATABASE_VERSION
            // TODO: and handle old database version in
            // TODO: handle_old_database_version() function.
            let table_structure = format!(
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
            );

            // Calculate table structure hash.
            let mut hasher = Sha512::new();
            hasher.update(&table_structure);
            let table_hash = hasher.finalize().to_vec();

            if table_hash != REPORT_TABLE_HASH {
                panic!("report table was changed and now is incompatible with old versions, \
                        to fix this panic, follow these steps:\n\
                        1. increment 'SUPPORTED_DATABASE_VERSION' constant\n\
                        2. handle old database version in 'handle_old_database_version()' function\n\
                        3. recalculate new table hash (sha512) and put it into 'REPORT_TABLE_HASH' constant.");
            }

            // Create table.
            let result = connection.execute(&table_structure, []);
            if let Err(e) = result {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
        }

        Ok(())
    }
    fn create_version_table_if_not_found(connection: &mut Connection) -> Result<(), AppError> {
        // Check if table exists.
        let mut stmt = connection
            .prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                VERSION_TABLE_NAME
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
            // TODO: IF CHANGING TABLE STRUCTURE,
            // TODO: remember to increment SUPPORTED_DATABASE_VERSION
            // TODO: and handle old database version in
            // TODO: handle_old_database_version() function.
            let table_structure = format!(
                "CREATE TABLE {}(
                    version INTEGER   
                )",
                VERSION_TABLE_NAME
            );

            // Calculate table structure hash.
            let mut hasher = Sha512::new();
            hasher.update(&table_structure);
            let table_hash = hasher.finalize().to_vec();

            if table_hash != VERSION_TABLE_HASH {
                panic!("version table was changed and now is incompatible with old versions, \
                        to fix this panic, follow these steps:\n\
                        1. increment 'SUPPORTED_DATABASE_VERSION' constant\n\
                        2. handle old database version in 'handle_old_database_version()' function\n\
                        3. recalculate new table hash (sha512) and put it into 'VERSION_TABLE_HASH' constant.");
            }

            // Create table.
            let result = connection.execute(&table_structure, []);
            if let Err(e) = result {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
        }

        // Insert database version.
        if let Err(e) = connection.execute(
            // password = hash(salt + hash(password))
            &format!(
                "INSERT INTO {} 
            (
                version
            ) 
            VALUES 
            (?1)",
                VERSION_TABLE_NAME
            ),
            params![SUPPORTED_DATABASE_VERSION],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

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
            // TODO: IF CHANGING TABLE STRUCTURE,
            // TODO: remember to increment SUPPORTED_DATABASE_VERSION
            // TODO: and handle old database version in
            // TODO: handle_old_database_version() function.
            let table_structure = format!(
                "CREATE TABLE {}(
                    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
                    username             TEXT NOT NULL UNIQUE,
                    salt                 TEXT NOT NULL,
                    password             TEXT NOT NULL,
                    need_change_password INTEGER NOT NULL,
                    need_setup_otp       INTEGER NOT NULL,
                    otp_secret_key       TEXT NOT NULL,
                    last_login_date      TEXT NOT NULL,
                    last_login_time      TEXT NOT NULL,
                    last_login_ip        TEXT NOT NULL,
                    date_registered      TEXT NOT NULL,
                    time_registered      TEXT NOT NULL
                )",
                USER_TABLE_NAME
            );

            // Calculate table structure hash.
            let mut hasher = Sha512::new();
            hasher.update(&table_structure);
            let table_hash = hasher.finalize().to_vec();

            if table_hash != USER_TABLE_HASH {
                panic!("user table was changed and now is incompatible with old versions, \
                        to fix this panic, follow these steps:\n\
                        1. increment 'SUPPORTED_DATABASE_VERSION' constant\n\
                        2. handle old database version in 'handle_old_database_version()' function\n\
                        3. recalculate new table hash (sha512) and put it into 'USER_TABLE_HASH' constant.");
            }

            let result = connection.execute(
                // password = hash(salt + hash(password))
                // need_change_password is '1' if the user
                // just registered, thus we need to ask him of a new password.
                &table_structure,
                [],
            );
            if let Err(e) = result {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
        }

        Ok(())
    }
    fn handle_old_database_version(connection: &mut Connection) -> Result<(), AppError> {
        // Get database version.
        let mut stmt = connection
            .prepare(&format!("SELECT version FROM {}", VERSION_TABLE_NAME))
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
            return Err(AppError::new("no version in database", file!(), line!()));
        }
        let row = row.unwrap();

        // Get version.
        let version: Result<u64> = row.get(0);
        if let Err(e) = version {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let version = version.unwrap();

        if version != SUPPORTED_DATABASE_VERSION {
            // TODO: handle old version here.
            // TODO: update old database to new format here.
            // TODO: ...
            // TODO: after everything is done, replace version value in the version table
            // TODO: with a new one.
            let notice_me = 42;
        }

        Ok(())
    }
}
