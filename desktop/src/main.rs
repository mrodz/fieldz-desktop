// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;

use anyhow::{Context, Result};
use bridge::*;
use db::Client;
use futures::lock::Mutex;
use std::sync::Arc;
use tauri::Manager;

#[derive(Debug, Default)]
struct SafeAppState(Arc<Mutex<State>>);

#[derive(Debug, Default)]
struct State {
    database: Option<Client>,
}

fn main() -> Result<()> {
    tauri::Builder::default()
        .manage(SafeAppState::default())
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();

            let db_path = app
                .path_resolver()
                .app_data_dir()
                .context("could not access app data directory")?;

            let state = app.state::<SafeAppState>();

            println!("Initializing...");

            let db_path = format!(
                "sqlite:{}?mode=rwc",
                db_path.join("data.sqlite").to_string_lossy()
            );

            println!("Using data: {db_path}");

            let db_config = db::Config::new(db_path);

            {
                let handle = tauri::async_runtime::handle();

                handle.block_on(async {
                    let mut guard = state.0.lock().await;
                    guard.database = Some(Client::new(&db_config).await?);
                    Result::<()>::Ok(())
                })?;
            }

            println!("Done initializing.");

            main_window.show().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_regions,
            create_region,
            delete_region,
            load_region,
            get_fields,
            create_field,
        ])
        .run(tauri::generate_context!())
        .context("error while running tauri application")
}
