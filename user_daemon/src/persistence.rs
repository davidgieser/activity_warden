use chrono::{DurationRound, Local, TimeDelta};
use rusqlite::params;
use directories::BaseDirs;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use log::{debug, info};
use sha2::{Sha256, Digest};

use shared::dbus::Host;
use shared::types::daemon::DurationMap;
use shared::types::schema::{AWTables, FocusChange, Password, QueryType, Timer};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;
use std::str::FromStr;

/// The name of the SQLite DB file used by the daemon.
const DB_FILE_NAME: &str = "aw_records.db3";
const STORE_DIR: &str = "activity_warden";
const PASSWORD_FILE_NAME: &str = "password_hash";

#[derive(Clone)]
pub struct PersistenceLayer {
    pool: Pool<SqliteConnectionManager>,
    /// The path to the raw SQL files.
    sql_root: PathBuf,
    /// The path to the user data directory where program files are stored.
    data_root: PathBuf,
}

impl PersistenceLayer {
    pub fn new() -> Self {
        let base_dirs = BaseDirs::new().unwrap();

        // Initialize the directory for the local file store.
        let mut data_root = base_dirs.data_dir().to_path_buf();
        data_root.push(STORE_DIR);
        if !data_root.exists() {
            fs::create_dir_all(&data_root).expect("Failed to initialize the local file store");
        }
        
        info!("Creating local file store at {:?}.", data_root);

        // Open the connection to the database from the local file.
        let db_path = data_root.join(DB_FILE_NAME);
        let manager = SqliteConnectionManager::file(&db_path);
        let pool = Pool::new(manager).expect("Failed to create sqlite pool");

        // Determine the path of the current file.
        let sql_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut this = PersistenceLayer { pool, sql_root, data_root };
        this.init_db();
        this
    }

    /// Initialize all database tables.
    fn init_db(&mut self) {
        let conn = self.pool.get().expect("Failed to obtain SQLite connection");
        for table in vec![AWTables::FocusChanges, AWTables::Timers] {
            if !conn.table_exists(None::<&str>, &table.to_string()).unwrap() {
                let create_query = self.load_sql(
                    &table, 
                    &QueryType::CREATE
                );
                conn.execute(&create_query, ()).unwrap();
                info!("Creating table {} in the DB.", table.to_string());
            } else {
                info!("Table {} already exists in the DB.", table.to_string());
            }
        }
    }

    fn load_sql(&self, table: &AWTables, action: &QueryType) -> String {
        let file_name = format!("{}_{}.sql", table, action);
        let mut path = self.sql_root.clone();
        
        // Queries reside in the `sql` subdirectory.
        path.push("sql");
        path.push(file_name);

        debug!("Loading a {} query for the {} table at path: {:?}.", action, table, path);
        fs::read_to_string(path).unwrap()
    }

    pub fn get_cur_password(&self) -> Option<Password> {
        debug!("[PASSWORD] Retrieving the active password.");

        let path = self.data_root.join(PASSWORD_FILE_NAME);
        if path.exists() {
            Some(fs::read_to_string(path).unwrap())
        } else {
            None
        }
    }

    /// Set a new password for the User Daemon. The password is hashed
    /// and written into a file.
    /// 
    /// Note that the password argument should be a raw, unhashed password.
    pub fn set_new_password(&self, password: Password) {
        let mut hasher = Sha256::new();
        hasher.update(password);
    
        let digest = hasher.finalize();
        let password_hash = hex::encode(digest);
        debug!("[PASSWORD] Setting the new password hash to {}.", password_hash);

        let path = self.data_root.join(PASSWORD_FILE_NAME);
        fs::write(path, password_hash).unwrap();
    }

    pub fn remove_password(&self) {
        debug!("[PASSWORD] Removing the password.");
        let path = self.data_root.join(PASSWORD_FILE_NAME);
        fs::remove_file(path).unwrap();
    }
    
    pub fn modify_timer(&self, action: QueryType, timer: Timer) {
        let conn = self.pool.get().expect("Failed to obtain SQLite connection.");
        let sql = self.load_sql(&AWTables::Timers, &action);
        
        let mut allowed_days = 0;
        for (i, &day) in timer.allowed_days.iter().enumerate() {
            if day {
                allowed_days |= 1 << i;
            }
        }

        conn.execute(
            &sql, 
            params![timer.display_name, timer.host.to_string(), timer.time_limit, allowed_days]
        ).expect("Failed to execute query");
    }

    pub fn select_timers(&self) -> Vec<Timer> {
        debug!("Attempting to select all timers.");

        let conn = self.pool.get().expect("Failed to obtain SQLite connection.");
        let sql = self.load_sql(&AWTables::Timers, &QueryType::SELECT);

        let mut stmt = conn.prepare(&sql).unwrap();
        let results = stmt.query_map([], |row| {
            Ok(Timer {
                display_name: row.get(0)?,
                host: row.get::<usize, String>(1)?.parse().unwrap(),
                time_limit: row.get(2)?,
                allowed_days: {
                    let value = row.get::<usize, u8>(3)?;
                    let mut allowed_days = Vec::new();
                    for i in 0..7 {
                        allowed_days.push((value & 1 << i) != 0);
                    }

                    allowed_days
                }
            })
        }).unwrap();

        results.into_iter().map(|v| v.unwrap()).collect()
    }

    pub fn select_current_durations(&self) -> DurationMap {
        let conn = self.pool.get().expect("Failed to obtain SQLite connection.");
        
        let sql = self.load_sql(&AWTables::FocusChanges, &QueryType::SELECT);
        let mut stmt = conn.prepare(&sql).unwrap();
        
        // Query using the local timezone to align with user expectations.
        let start_day = Local::now()
            .duration_trunc(TimeDelta::days(1))
            .unwrap();
        let end_day = start_day + chrono::Duration::days(1);
        let mut rows = stmt.query([
            start_day,
            end_day
        ]).unwrap();
        let mut durations: DurationMap = HashMap::new();

        while let Some(row) = rows.next().unwrap() {
            let display_name: String = row.get(0).unwrap();
            let host: String = row.get(1).unwrap();
            let dur: u32 = row.get(2).unwrap();

            let host_map = durations.entry(Host::from_str(&host).unwrap()).or_default();
            let display_name_duration = host_map.entry(display_name).or_default();
            *display_name_duration += dur;
        }

        durations
    }

    pub fn insert_focus_change(&self, fc: &FocusChange) {
        let conn = self.pool.get().expect("Failed to obtain SQLite connection.");
        
        let sql = self.load_sql(&AWTables::FocusChanges, &QueryType::INSERT);
        conn.execute(
            &sql, 
            params![fc.display_name, fc.host.to_string(), fc.timestamp, fc.duration]
        ).expect("Failed to execute query");
    }
}