// External.
use chrono::prelude::*;
use rand::Rng;
use rusqlite::{params, Connection, Result};
use sha2::{Digest, Sha512};

// Custom.
use super::report::*;
use crate::misc::error::AppError;

pub const DATABASE_NAME: &str = "database.db3";

const REPORT_TABLE_NAME: &str = "report";
const USER_TABLE_NAME: &str = "user";
const ATTACHMENT_TABLE_NAME: &str = "attachment";
const VERSION_TABLE_NAME: &str = "version";

const REPORT_TABLE_HASH: &[u8] = &[
    158, 21, 8, 202, 214, 61, 94, 63, 56, 50, 126, 200, 244, 198, 125, 37, 0, 153, 116, 141, 15,
    11, 48, 160, 50, 110, 165, 128, 140, 85, 196, 95, 193, 74, 107, 154, 149, 99, 110, 89, 146,
    124, 113, 35, 202, 214, 246, 205, 201, 243, 112, 11, 110, 0, 72, 42, 46, 178, 157, 141, 234,
    148, 214, 28,
];
const USER_TABLE_HASH: &[u8] = &[
    179, 199, 233, 204, 132, 161, 204, 15, 152, 12, 233, 72, 42, 79, 252, 183, 189, 251, 215, 202,
    231, 239, 91, 23, 60, 246, 24, 163, 210, 30, 170, 135, 202, 217, 94, 240, 145, 21, 214, 49, 30,
    222, 23, 219, 226, 1, 250, 93, 145, 60, 222, 228, 75, 190, 7, 226, 183, 74, 167, 19, 26, 142,
    161, 64,
];
const ATTACHMENT_TABLE_HASH: &[u8] = &[
    93, 157, 12, 163, 145, 37, 18, 255, 0, 224, 239, 160, 43, 13, 61, 12, 238, 220, 239, 125, 195,
    173, 133, 252, 189, 49, 48, 195, 211, 138, 4, 122, 22, 114, 178, 251, 159, 196, 114, 234, 5,
    206, 193, 119, 119, 140, 106, 6, 44, 56, 184, 114, 190, 215, 142, 70, 38, 27, 98, 115, 88, 138,
    125, 139,
];

const SUPPORTED_DATABASE_VERSION: u64 = 1;

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

pub struct ReportData {
    pub id: u64,
    pub title: String,
    pub game_name: String,
    pub game_version: String,
    pub text: String,
    pub date: String,
    pub time: String,
    pub sender_name: String,
    pub sender_email: String,
    pub os_info: String,
}

pub struct DatabaseManager {
    connection: Connection,
}

impl DatabaseManager {
    /// Open a new database connection.
    /// If no database was created, will create a new one.
    pub fn new() -> Result<Self, AppError> {
        let sqlite_version = rusqlite::version_number();
        if sqlite_version < 3035000 {
            // because we use RETURNING clause
            panic!(
                "Used SQLite version \"{}\" is not supported,
                minimum supported version of SQLite is \"3.35.0\".",
                rusqlite::version()
            )
        }

        let result = Connection::open(DATABASE_NAME);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut connection = result.unwrap();

        // Enable foreign keys.
        if let Some(app_error) = Self::enable_foreign_keys(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'version' table.
        if let Err(app_error) = Self::create_version_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'report' table.
        if let Err(app_error) = Self::create_report_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'user' table.
        if let Err(app_error) = Self::create_user_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check 'attachment' table.
        if let Err(app_error) = Self::create_attachment_table_if_not_found(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Handle old database version.
        if let Err(app_error) = Self::handle_old_database_version(&mut connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(Self { connection })
    }
    /// Returns the amount of reports the database contains.
    pub fn get_report_count(&self) -> Result<u64, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!("SELECT count(id) FROM {}", REPORT_TABLE_NAME))
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
            return Err(AppError::new("database returned none", file!(), line!()));
        } else {
            let count = row.unwrap().get(0);
            if let Err(e) = count {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            return Ok(count.unwrap());
        }
    }
    /// Returns summary of reports from the database.
    ///
    /// Parameters:
    /// - `page`: a "page" to query reports from
    /// - `amount`: amount of reports to query
    ///
    /// In the database reports exist as a "list"
    /// to implement "paging" in client application we use 2 values:
    /// `page` and `amount` when querying reports. To query reports
    /// we calculate starting id as `(page - 1) * amount` and select
    /// `amount` rows starting from this starting id.
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
    /// Returns a report with the specified ID from the database.
    ///
    /// Returns error if a report with the specified ID does not exist.
    pub fn get_report(&self, report_id: u64) -> Result<ReportData, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT id, report_name, report_text, sender_name, sender_email, \
                game_name, game_version, os_info, date_created_at, time_created_at \
                FROM {} WHERE id == {}",
                REPORT_TABLE_NAME, report_id
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
                &format!("report with id '{}' was not found", report_id),
                file!(),
                line!(),
            ));
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

        // Get report text.
        let text = row.get(2);
        if let Err(e) = text {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let text: String = text.unwrap();

        // Get report sender name.
        let sender_name = row.get(3);
        if let Err(e) = sender_name {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let sender_name: String = sender_name.unwrap();

        // Get report sender email.
        let sender_email = row.get(4);
        if let Err(e) = sender_email {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let sender_email: String = sender_email.unwrap();

        // Get report game name.
        let game_name = row.get(5);
        if let Err(e) = game_name {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let game_name: String = game_name.unwrap();

        // Get report game version.
        let game_version = row.get(6);
        if let Err(e) = game_version {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let game_version: String = game_version.unwrap();

        // Get reporter OS info.
        let os_info = row.get(7);
        if let Err(e) = os_info {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let os_info: String = os_info.unwrap();

        // Get report date.
        let date = row.get(8);
        if let Err(e) = date {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let date: String = date.unwrap();

        // Get report time.
        let time = row.get(9);
        if let Err(e) = time {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let time: String = time.unwrap();

        Ok(ReportData {
            id,
            title,
            game_name,
            game_version,
            text,
            date,
            time,
            sender_name,
            sender_email,
            os_info,
        })
    }
    /// Adds a new user to the database.
    ///
    /// Parameters:
    /// - `username` login of the new user
    /// - `is_admin` whether the user should have admin privileges or not
    /// (be able to delete reports using the client application).
    pub fn add_user(&self, username: &str, is_admin: bool) -> AddUserResult {
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
                is_admin,
                last_login_date,
                last_login_time,
                last_login_ip,
                date_registered,
                time_registered
            ) 
            VALUES 
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                USER_TABLE_NAME
            ),
            params![
                username,
                salt,
                password,
                1, // change password
                1, // have not received OTP QR code
                otp_secret,
                is_admin,
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
            // TODO: handle non existent user here
            // password = hash(salt + hash(password))
            &format!(
                "DELETE FROM {}
                WHERE username == '{}'",
                USER_TABLE_NAME, username
            ),
            params![],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(true)
    }
    /// Removes a report from the database.
    ///
    /// Returns `Ok(true)` if the report was found and removed,
    /// `Ok(false)` if the report was not found.
    /// On failure returns error description via `AppError`.
    pub fn remove_report(&self, report_id: u64) -> Result<bool, AppError> {
        let result = self.is_report_exists(report_id);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let exists = result.unwrap();
        if exists == false {
            return Ok(false);
        }

        // Remove report.
        if let Err(e) = self.connection.execute(
            // TODO: handle non existent report here
            &format!(
                "DELETE FROM {}
                WHERE id == {}",
                REPORT_TABLE_NAME, report_id
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
    /// Saves a new report to the database.
    pub fn save_report(
        &self,
        game_report: GameReport,
        attachments: Vec<ReportAttachment>,
    ) -> Result<(), AppError> {
        // Insert report into the database.
        let datetime = Local::now();
        let result: Result<u64> = self.connection.query_row(
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
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) 
            RETURNING id",
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
                datetime.time().format("%H:%M:%S").to_string(),
            ],
            |row| row.get(0),
        );
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let report_id = result.unwrap();

        // Insert report attachments into the database.
        for attachment in attachments {
            let data_size_in_bytes = attachment.data.len();
            let result = self.connection.execute(
                &format!(
                    "INSERT INTO {} 
                    (
                        file_name,
                        data, 
                        size_in_bytes,
                        fk_report_id
                    ) 
                    VALUES 
                    (?1, ?2, ?3, ?4)",
                    ATTACHMENT_TABLE_NAME
                ),
                params![
                    attachment.file_name,
                    attachment.data,
                    data_size_in_bytes,
                    report_id
                ],
            );
            if let Err(e) = result {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
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
    /// Check if a given user has admin privileges.
    ///
    /// Returns `Ok(true)` if yes, `Ok(false)` if no.
    /// On failure returns `AppError`.
    pub fn is_user_admin(&self, username: &str) -> Result<bool, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT is_admin FROM {} WHERE username='{}'",
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
        let is_admin = row.get(0);
        if let Err(e) = is_admin {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let is_admin: i32 = is_admin.unwrap();

        if is_admin == 1 {
            return Ok(true);
        } else if is_admin == 0 {
            return Ok(false);
        } else {
            return Err(AppError::new(
                &format!(
                    "database returned 'is_admin' equal to '{}' for user {}",
                    is_admin, username
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
    /// Enabling foreign keys protects us from violating foreign key constraints
    /// and also enables ON DELETE CASCADE logic.
    fn enable_foreign_keys(connection: &mut Connection) -> Option<AppError> {
        let result = connection.execute("PRAGMA foreign_keys=on;", []);
        if let Err(e) = result {
            return Some(AppError::new(&e.to_string(), file!(), line!()));
        }

        None
    }
    /// Check if a given report exists in the database.
    ///
    /// Returns `Ok(true)` if the report exists, `Ok(false)` if not.
    /// On failure returns `AppError`.
    fn is_report_exists(&self, report_id: u64) -> Result<bool, AppError> {
        let mut stmt = self
            .connection
            .prepare(&format!(
                "SELECT id FROM {} WHERE id='{}'",
                REPORT_TABLE_NAME, report_id
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
    /// Creates the `report` table if it was not found in the database.
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

        // Create this table.
        // 'attachments' below is a string of IDs from 'attachment' table
        // separated by spaces
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
                    time_created_at TEXT NOT NULL,
                    attachments     TEXT    
                )",
            REPORT_TABLE_NAME
        );

        // Calculate table structure hash.
        let mut hasher = Sha512::new();
        hasher.update(&table_structure);
        let table_hash = hasher.finalize().to_vec();

        if table_hash != REPORT_TABLE_HASH {
            panic!("\"report\" table was changed and now is incompatible with old versions, \
                        to fix this panic, follow these steps:\n\
                        1. increment 'SUPPORTED_DATABASE_VERSION' constant\n\
                        2. handle old database version in 'handle_old_database_version()' function\n\
                        3. recalculate new table hash (sha512) and put it into 'REPORT_TABLE_HASH' constant.");
        }

        if row.is_some() {
            return Ok(());
        }

        // Create table.
        let result = connection.execute(&table_structure, []);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    /// Creates the `version` table if it was not found in the database.
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

        if row.is_some() {
            return Ok(());
        }

        let table_structure = format!(
            "CREATE TABLE {}(
                    version INTEGER   
                )",
            VERSION_TABLE_NAME
        );

        // Create table.
        let result = connection.execute(&table_structure, []);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        // Insert database version.
        if let Err(e) = connection.execute(
            // password = hash(salt + hash(password))
            &format!("INSERT INTO {} (version) VALUES (?1)", VERSION_TABLE_NAME),
            params![SUPPORTED_DATABASE_VERSION],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        println!("INFO: database version is {}", SUPPORTED_DATABASE_VERSION);

        Ok(())
    }
    /// Creates the `user` table if it was not found in the database.
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

        // Create this table.
        // password = hash(salt + hash(password))
        // need_change_password is '1' if the user
        // just registered, thus we need to ask him of a new password.
        let table_structure = format!(
            "CREATE TABLE {}(
                    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
                    username             TEXT NOT NULL UNIQUE,
                    salt                 TEXT NOT NULL,
                    password             TEXT NOT NULL,
                    need_change_password INTEGER NOT NULL,
                    need_setup_otp       INTEGER NOT NULL,
                    otp_secret_key       TEXT NOT NULL,
                    is_admin             INTEGER NOT NULL,
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
            panic!("\"user\" table was changed and now is incompatible with old versions, \
                        to fix this panic, follow these steps:\n\
                        1. increment 'SUPPORTED_DATABASE_VERSION' constant\n\
                        2. handle old database version in 'handle_old_database_version()' function\n\
                        3. recalculate new table hash (sha512) and put it into 'USER_TABLE_HASH' constant.");
        }

        if row.is_some() {
            return Ok(());
        }

        let result = connection.execute(&table_structure, []);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    /// Creates the `attachment` table if it was not found in the database.
    fn create_attachment_table_if_not_found(connection: &mut Connection) -> Result<(), AppError> {
        // Check if table exists.
        let mut stmt = connection
            .prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                ATTACHMENT_TABLE_NAME
            ))
            .unwrap();
        let result = stmt.query([]);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let mut rows = result.unwrap();
        let row = rows.next().unwrap();

        // Create this table.
        let table_structure = format!(
            "CREATE TABLE {}(
                    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_name            TEXT NOT NULL,
                    data                 BLOB NOT NULL,
                    size_in_bytes        INTEGER NUL NULL,
                    fk_report_id         INTEGER NOT NULL,
                    FOREIGN KEY (fk_report_id) REFERENCES report (id) ON DELETE CASCADE
                )",
            ATTACHMENT_TABLE_NAME
        );

        // Calculate table structure hash.
        let mut hasher = Sha512::new();
        hasher.update(&table_structure);
        let table_hash = hasher.finalize().to_vec();

        if table_hash != ATTACHMENT_TABLE_HASH {
            panic!("\"attachment\" table was changed and now is incompatible with old versions, \
                        to fix this panic, follow these steps:\n\
                        1. increment 'SUPPORTED_DATABASE_VERSION' constant\n\
                        2. handle old database version in 'handle_old_database_version()' function\n\
                        3. recalculate new table hash (sha512) and put it into 'ATTACHMENT_TABLE_HASH' constant.");
        }

        if row.is_some() {
            return Ok(());
        }

        let result = connection.execute(&table_structure, []);
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    /// Looks if the existing database is not supported by this database manager.
    /// If the existing database is not supported, will upgrade existing database
    /// to the currently supported version.
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

        if version == SUPPORTED_DATABASE_VERSION {
            return Ok(());
        }

        println!(
            "INFO: current database version {} is old, updating to the latest database version {}...",
            version, SUPPORTED_DATABASE_VERSION
        );

        drop(row);
        drop(rows);
        drop(stmt);

        if version == 0 {
            // Upgrade to version 1.
            if let Err(app_error) = DatabaseManager::upgrade_database_to_version_1(connection) {
                return Err(app_error.add_entry(file!(), line!()));
            }
        }

        // Handle old version here.
        // Upgrade old database to the new format here.
        //
        // ... upgrade code here under if version == ...
        //

        // After everything is done, we drop the version table
        // and insert the new version value.
        if let Err(app_error) = DatabaseManager::drop_version_table(connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }
        if let Err(app_error) = DatabaseManager::create_version_table_if_not_found(connection) {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(())
    }
    /// Deletes the `version` table.
    fn drop_version_table(connection: &mut Connection) -> Result<(), AppError> {
        if let Err(e) = connection.execute(&format!("DROP TABLE {}", VERSION_TABLE_NAME), params![])
        {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    /// Upgrades the database from version `0` to version `1`.
    fn upgrade_database_to_version_1(connection: &mut Connection) -> Result<(), AppError> {
        if let Err(e) = connection.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0",
                USER_TABLE_NAME
            ),
            params![],
        ) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
}
