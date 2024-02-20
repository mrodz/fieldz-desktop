// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Context;
use db::Client;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Debug, Default)]
struct SafeAppState(Mutex<State>);

#[derive(Debug, Default)]
struct State {
    database: Option<Client>,
}

fn main() {
    tauri::Builder::default()
        .manage(SafeAppState::default())
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();

            let db_path = app
                .path_resolver()
                .app_data_dir()
                .context("could not access app data directory")?;

            let state = app.state::<SafeAppState>();

            // initialize your app here instead of sleeping :)
            println!("Initializing...");

            let db_path = format!("sqlite:{}data.sqlite?mode=rwc", db_path.to_string_lossy());

            println!("Using data: {db_path}");

            let db_config = db::Config::new(db_path);

            {
                let handle = tauri::async_runtime::handle();
                let mut guard = state.0.lock().unwrap();
                guard.database = Some(handle.block_on(async { Client::new(&db_config).await })?);
            }

            println!("Done initializing.");

            main_window.show().unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
