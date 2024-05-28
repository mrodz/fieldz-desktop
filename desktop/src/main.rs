// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod net;

use anyhow::{bail, Context, Result};
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
    if dotenv::dotenv().is_err() {
        println!("Not using any environment variables in a `.env` file.")
    };

    static NONE_WINDOW: Option<&'static tauri::Window> = None;

    match std::panic::catch_unwind(|| {
        tauri::Builder::default()
            .manage(SafeAppState::default())
            .plugin(tauri_plugin_oauth::init())
            .setup(|app| {
                let main_window = app.get_window("main").unwrap();

                let db_path = app
                    .path_resolver()
                    .app_data_dir()
                    .context("could not access app data directory")?;

                let dir_part = if db_path.is_file() {
                    db_path
                        .parent()
                        .expect("path to database is not a file, and it has no parent directory")
                } else {
                    db_path.as_path()
                };

                // Ensure path to database exists
                std::fs::create_dir_all(dir_part)?;

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
                delete_field,
                create_team,
                get_teams,
                delete_team,
                db_migrate_up_down,
                create_group,
                get_groups,
                delete_group,
                get_teams_and_tags,
                get_time_slots,
                create_time_slot,
                get_field,
                move_time_slot,
                delete_time_slot,
                list_reservations_between,
                load_all_teams,
                update_region,
                update_team,
                get_targets,
                create_target,
                delete_target,
                target_add_group,
                target_delete_group,
                generate_pre_schedule_report,
                create_reservation_type,
                get_reservation_types,
                delete_reservation_type,
                update_reservation_type,
                get_supported_concurrency_for_field,
                update_reservation_type_concurrency_for_field,
                get_non_default_reservation_type_concurrency_associations,
                update_target_reservation_type,
                generate_schedule_payload,
                schedule,
                get_schedules,
                delete_schedule,
                update_schedule,
                get_schedule,
                health_probe,
                get_schedule_games,
                get_team,
                get_scheduler_url,
            ])
            .run(tauri::generate_context!())
            .context("error while running tauri application")
    }) {
        Err(e) => {
            let info = e.downcast_ref::<&str>();
            static PANIC_MSG: &str = "unable to determine error information";
            tauri::api::dialog::message(NONE_WINDOW, "Error", info.unwrap_or(&PANIC_MSG));
            bail!("the app crashed in a panicked state");
        }
        Ok(err @ Err(..)) => err.inspect_err(|e| {
            tauri::api::dialog::message(NONE_WINDOW, "Error", format!("{e}"));
        }),
        Ok(Ok(_)) => Ok(()),
    }
}
