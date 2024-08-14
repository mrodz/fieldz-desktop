// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod bridge;
mod net;

use anyhow::{Context, Result};
use bridge::*;
use db::Client;
use futures::lock::Mutex;
use std::sync::Arc;
use tauri::{Manager, Wry};
use tauri_plugin_store::{Store, StoreBuilder};

#[derive(Debug, Default)]
struct SafeAppState(Arc<Mutex<State>>);

#[derive(Default)]
struct State {
    database: Option<Client>,
    connection_pool: Option<reqwest::Client>,
    metadata_store: Option<Store<Wry>>,
}

pub(crate) const ACTIVE_PROFILE_BIN_KEY: &str = "active_profile";

fn main() -> Result<()> {
    if dotenv::dotenv().is_err() {
        println!("Not using any environment variables in a `.env` file.")
    };

    tauri::Builder::default()
        .manage(SafeAppState::default())
        .plugin(auth::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();

            let db_path = app
                .path_resolver()
                .app_data_dir()
                .context("could not access app data directory")?;

            let dir_part = if db_path.is_file() {
                db_path
                    .parent()
                    .context("path to database is not a file, and it has no parent directory")?
            } else {
                db_path.as_path()
            };

            // Ensure path to database exists
            std::fs::create_dir_all(dir_part)?;

            // Ensure path to profiles exists
            let profiles_directory = dir_part.join("profiles");
            std::fs::create_dir_all(&profiles_directory)?;

            let metadata_path = dir_part.join("metadata.bin");

            let mut store = StoreBuilder::new(app.handle(), metadata_path.clone()).build();

            if metadata_path.try_exists()? {
                store.load()?;
            }

            let active_profile = store.get(ACTIVE_PROFILE_BIN_KEY);

            println!("Last selected profile: {active_profile:?}");

            let db_path = match active_profile {
                None | Some(serde_json::Value::Null) => db_path,
                Some(name_of_dir) => {
                    let active_profile_directory = profiles_directory
                        .join(name_of_dir.as_str().context("`name_of_dir` is not a string")?);
                    if active_profile_directory.try_exists().context("Could not check if the profile directory exists")? {
                        active_profile_directory
                    } else {
                        db_path
                    }
                }
            };

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
                    guard.database = Some(Client::new(&db_config).await.context("Could not connect to the data source. Is it missing? Please update Fieldz as soon as possible as your data may be out of sync.")?);
                    guard.connection_pool = Some(reqwest::Client::new());
                    guard.metadata_store = Some(store);
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
            get_github_access_token,
            generate_code_challenge,
            copy_time_slots,
            delete_time_slots_batched,
            create_coaching_conflict,
            delete_coaching_conflict,
            coaching_conflict_team_op,
            coaching_conflict_rename,
            get_coach_conflicts,
            get_region_metadata,
            begin_twitter_oauth_transaction,
            finish_twitter_oauth_transaction,
            list_profiles,
            get_active_profile,
            set_active_profile,
            create_new_profile,
            delete_profile,
            rename_profile,
            set_reservation_type_practice,
            swap_schedule_games,
        ])
        .run(tauri::generate_context!())
        .inspect_err(|e| {
            tauri::api::dialog::blocking::message(None::<&'static tauri::Window>, "Error", format!("{e:?}"));
        })
        .context("error while running tauri application")
}
