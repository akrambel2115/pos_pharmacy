pub mod migrations;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

pub struct DbState {
    pub conn: Mutex<Connection>,
}

pub fn init_db(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    // Resolve AppData folder
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;

    let db_path = app_dir.join("pharmacy.db");
    println!("Database path: {:?}", db_path);

    // Open connection
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

    // Run migrations
    migrations::run_migrations(&conn)
        .map_err(|e| format!("Database migration failed: {}", e))?;

    Ok(conn)
}
