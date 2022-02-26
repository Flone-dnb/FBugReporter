// Std.
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

// External.
use chrono::{DateTime, Local};

// Custom.
use crate::services::logger_service::Logger;

/// If a user failed to login more than this value times,
/// he will be banned.
/// # Warning
/// This value should not be zero.
pub const MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN: u32 = 3;
/// The amount of time a banned user will have to wait
/// until he can try to login again.
/// # Warning
/// This value should not be zero or negative.
pub const BAN_TIME_DURATION_IN_MIN: i64 = 5;

pub enum AttemptResult {
    Fail { attempts_made: u32 },
    Ban,
}

/// This struct represents an IP address of a
/// client who failed to login.
/// New failed attempts will cause the client's IP
/// to be banned.
#[derive(Debug)]
pub struct FailedIP {
    pub ip: IpAddr,
    pub failed_attempts_made: u32,
    pub last_attempt_time: DateTime<Local>,
}

/// This struct represents an IP address of a
/// client who failed to login multiple times.
#[derive(Debug)]
pub struct BannedIP {
    pub ip: IpAddr,
    pub ban_start_time: DateTime<Local>,
}
pub struct BanManager {
    failed_ip_list: Mutex<Vec<FailedIP>>,
    banned_ip_list: Mutex<Vec<BannedIP>>,
    logger: Arc<Mutex<Logger>>,
}

impl BanManager {
    pub fn new(logger: Arc<Mutex<Logger>>) -> Self {
        if MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN == 0 {
            panic!("MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN could not be zero.");
        }
        if BAN_TIME_DURATION_IN_MIN < 0 {
            panic!("BAN_TIME_DURATION_IN_MIN could not be zero or negative.");
        }
        Self {
            failed_ip_list: Mutex::new(Vec::new()),
            banned_ip_list: Mutex::new(Vec::new()),
            logger,
        }
    }
    /// Adds a failed login attempt to the IP.
    /// If this IP failed more than `MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN`
    /// it will be banned and removed from the failed ips list.
    ///
    /// Returns `AttemptResult` that shows the current IP state (failed login / banned).
    pub fn add_failed_login_attempt(&mut self, username: &str, ip: IpAddr) -> AttemptResult {
        let mut failed_ip_guard = self.failed_ip_list.lock().unwrap();

        // Find in failed_ip_list.
        let found_pos = failed_ip_guard.iter().position(|x| x.ip == ip);
        let mut failed_attempts_made: u32 = 0;
        if found_pos.is_some() {
            failed_attempts_made = failed_ip_guard[found_pos.unwrap()].failed_attempts_made;
        }

        // Add current failed attempt.
        failed_attempts_made += 1;

        if failed_attempts_made > MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN {
            // Add to banned ips.
            if found_pos.is_some() {
                // Remove from failed ip list.
                failed_ip_guard.remove(found_pos.unwrap());
            }

            let mut banned_ips_guard = self.banned_ip_list.lock().unwrap();
            banned_ips_guard.push(BannedIP {
                ip,
                ban_start_time: Local::now(),
            });

            self.logger.lock().unwrap().print_and_log(&format!(
                "{} was banned for {} minute(-s) due to {} failed login attempts.",
                username, BAN_TIME_DURATION_IN_MIN, failed_attempts_made
            ));

            AttemptResult::Ban
        } else {
            if found_pos.is_some() {
                let failed_ip = failed_ip_guard.get_mut(found_pos.unwrap()).unwrap();
                failed_ip.failed_attempts_made = failed_attempts_made;
            } else {
                failed_ip_guard.push(FailedIP {
                    ip,
                    failed_attempts_made,
                    last_attempt_time: Local::now(),
                });
            }

            self.logger.lock().unwrap().print_and_log(&format!(
                "{} failed to login: {}/{} allowed failed login attempts.",
                username, failed_attempts_made, MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN
            ));

            AttemptResult::Fail {
                attempts_made: failed_attempts_made,
            }
        }
    }
    /// Removes old entries, this means:
    /// - failed IP will be removed only if `last_attempt_time` was made
    /// `BAN_TIME_DURATION_IN_MIN` ago or longer,
    /// - banned IP will be removed only if `ban_start_time` was
    /// `BAN_TIME_DURATION_IN_MIN` ago or longer.
    pub fn refresh_failed_and_banned_lists(&mut self) {
        let mut failed_list_guard = self.failed_ip_list.lock().unwrap();
        let mut banned_list_guard = self.banned_ip_list.lock().unwrap();

        // Refresh failed ips list.
        let mut _failed_list_len_before = failed_list_guard.len();

        failed_list_guard.retain(|ip| {
            let time_diff = Local::now() - ip.last_attempt_time;
            time_diff.num_minutes() < BAN_TIME_DURATION_IN_MIN
        });

        // Refresh banned ips list.
        let mut _banned_list_len_before = banned_list_guard.len();

        banned_list_guard.retain(|ip| {
            let time_diff = Local::now() - ip.ban_start_time;
            time_diff.num_minutes() < BAN_TIME_DURATION_IN_MIN
        });

        // Log if anything changed.
        if _failed_list_len_before != failed_list_guard.len()
            || _banned_list_len_before != banned_list_guard.len()
        {
            self.logger.lock().unwrap().print_and_log(&format!(
                "Refreshed failed and banned ip lists to remove old entries:\n\
                    before:\n\
                    - failed ip list size: {}\n\
                    - banned ip list size: {}\n\
                    after:\n\
                    - failed ip list size: {}\n\
                    - banned ip list size: {}.",
                _failed_list_len_before,
                _banned_list_len_before,
                failed_list_guard.len(),
                banned_list_guard.len()
            ));
        }
    }
    /// Checks if the specified IP is in the ban list.
    /// If the specified IP is in the ban list, this function will also
    /// check if the ban time has passed and the IP is no longer banned.
    pub fn is_ip_banned(&self, ip: IpAddr) -> bool {
        let mut banned_list_guard = self.banned_ip_list.lock().unwrap();
        let is_banned = banned_list_guard.iter().position(|x| x.ip == ip);

        if is_banned.is_some() {
            // This IP is banned, see if ban time is over.
            let banned_ip_index = is_banned.unwrap();
            let time_diff = Local::now() - banned_list_guard[banned_ip_index].ban_start_time;

            if time_diff.num_minutes() < BAN_TIME_DURATION_IN_MIN {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "Banned IP address ({}) attempted to connect. \
                            Connection was rejected.",
                    ip.to_string()
                ));
                return true; // still banned
            } else {
                // Remove from banned ips.
                banned_list_guard.remove(banned_ip_index);
                return false;
            }
        } else {
            // Check if user failed to login before.
            let mut failed_list_guard = self.failed_ip_list.lock().unwrap();
            let failed_before = failed_list_guard.iter().position(|x| x.ip == ip);

            if failed_before.is_some() {
                // See if we can remove this ip from failed ips
                // if the last failed attempt was too long ago.
                let failed_index = failed_before.unwrap();
                let time_diff = Local::now() - failed_list_guard[failed_index].last_attempt_time;

                if time_diff.num_minutes() >= BAN_TIME_DURATION_IN_MIN {
                    failed_list_guard.remove(failed_index);
                }
            }

            return false;
        }
    }
    /// Removes the specified IP from the failed ips list.
    pub fn remove_ip_from_failed_ips_list(&mut self, ip: IpAddr) {
        let mut failed_ip_list_guard = self.failed_ip_list.lock().unwrap();

        let index_to_remove = failed_ip_list_guard.iter().position(|x| x.ip == ip);
        if index_to_remove.is_some() {
            failed_ip_list_guard.remove(index_to_remove.unwrap());
        } // else: user made no failed login attempts before
    }
}
