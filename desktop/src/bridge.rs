use backend::ScheduledInput;
use base64::Engine;
use db::errors::{
    CreateFieldError, CreateGroupError, CreateRegionError, CreateReservationTypeError,
    CreateTeamError, DeleteFieldError, DeleteGroupError, DeleteRegionError, DeleteTeamError,
    EditRegionError, EditScheduleError, EditTeamError, GetScheduledInputsError, LoadFieldsError,
    LoadRegionError, LoadScheduleError, LoadTeamsError, PreScheduleReportError, TimeSlotError,
};
use db::{
    CreateFieldInput, CreateRegionInput, CreateReservationTypeInput, CreateTeamInput,
    CreateTimeSlotInput, EditRegionInput, EditScheduleInput, EditTeamInput, FieldConcurrency,
    FieldExtension, FieldSupportedConcurrencyInput, ListReservationsBetweenInput,
    MoveTimeSlotInput, PreScheduleReport, PreScheduleReportInput, TargetExtension, TeamCollection,
    TeamExtension, TimeSlotExtension, UpdateReservationTypeConcurrencyForFieldInput,
    UpdateTargetReservationTypeInput, Validator,
};
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use crate::net::{
    self, send_grpc_schedule_request, HealthProbeError, OAuthAccessTokenExchange, ScheduleRequestError, ServerHealth
};
use crate::SafeAppState;

#[tauri::command]
pub(crate) async fn get_regions(app: AppHandle) -> Result<Vec<db::region::Model>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client.get_regions().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn create_region(
    app: AppHandle,
    input: CreateRegionInput,
) -> Result<db::region::Model, CreateRegionError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CreateRegionError::NoDatabase)?;

    input
        .validate()
        .map_err(CreateRegionError::ValidationError)?;

    client
        .create_region(input)
        .await
        .map_err(|e| CreateRegionError::DatabaseError(e.to_string()))
}

#[tauri::command]
pub(crate) async fn delete_region(app: AppHandle, id: i32) -> Result<(), DeleteRegionError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(DeleteRegionError::NoDatabase)?;

    let deletion = client.delete_region(id).await;

    match deletion {
        Ok(deletion) => {
            if deletion.rows_affected == 1 {
                Ok(())
            } else {
                Err(DeleteRegionError::NotFound(id))
            }
        }
        Err(e) => Err(DeleteRegionError::DatabaseError(e.to_string())),
    }
}

#[tauri::command]
pub(crate) async fn load_region(
    app: AppHandle,
    id: i32,
) -> Result<db::region::Model, LoadRegionError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadRegionError::NoDatabase)?;

    let deletion = client.load_region(id).await;

    match deletion {
        Ok(deletion) => {
            if deletion.len() == 1 {
                Ok(deletion[0].clone())
            } else {
                Err(LoadRegionError::NotFound(id))
            }
        }
        Err(e) => Err(LoadRegionError::DatabaseError(e.to_string())),
    }
}

#[tauri::command]
pub(crate) async fn get_fields(
    app: AppHandle,
    region_id: i32,
) -> Result<Vec<db::field::Model>, LoadFieldsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadFieldsError::NoDatabase)?;

    client
        .get_fields(region_id)
        .await
        .map_err(|e| LoadFieldsError::DatabaseError(e.to_string()))
}

#[tauri::command]
pub(crate) async fn get_field(
    app: AppHandle,
    field_id: i32,
) -> Result<db::field::Model, LoadFieldsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadFieldsError::NoDatabase)?;

    match client
        .get_field(field_id)
        .await
        .map_err(|e| LoadFieldsError::DatabaseError(e.to_string()))
    {
        Ok(mut fields) if fields.len() == 1 => Ok(fields.remove(0)),
        Ok(..) => Err(LoadFieldsError::DatabaseError(
            "too many/little fields".to_owned(),
        )),
        Err(e) => Err(LoadFieldsError::DatabaseError(e.to_string())),
    }
}

#[tauri::command]
pub(crate) async fn create_field(
    app: AppHandle,
    input: CreateFieldInput,
) -> Result<db::field::Model, CreateFieldError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(CreateFieldError::NoDatabase)?;

    input
        .validate()
        .map_err(CreateFieldError::ValidationError)?;

    client
        .create_field(input)
        .await
        .map_err(|e| CreateFieldError::DatabaseError(e.to_string()))
}

#[tauri::command]
pub(crate) async fn delete_field(app: AppHandle, id: i32) -> Result<(), DeleteFieldError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(DeleteFieldError::NoDatabase)?;

    let deletion = client.delete_field(id).await;

    match deletion {
        Ok(deletion) => {
            if deletion.rows_affected == 1 {
                Ok(())
            } else {
                Err(DeleteFieldError::NotFound(id))
            }
        }
        Err(e) => Err(DeleteFieldError::DatabaseError(e.to_string())),
    }
}

#[tauri::command]
pub(crate) async fn get_teams(
    app: AppHandle,
    region_id: i32,
) -> Result<Vec<db::team::Model>, LoadTeamsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadTeamsError::NoDatabase)?;

    client
        .get_teams(region_id)
        .await
        .map_err(|e| LoadTeamsError::DatabaseError(e.to_string()))
}

#[tauri::command]
pub(crate) async fn get_teams_and_tags(
    app: AppHandle,
    region_id: i32,
) -> Result<Vec<db::TeamExtension>, LoadTeamsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadTeamsError::NoDatabase)?;

    client
        .get_teams_with_tags(region_id)
        .await
        .map_err(|e| LoadTeamsError::DatabaseError(e.to_string()))
}

#[tauri::command]
pub(crate) async fn create_team(
    app: AppHandle,
    input: CreateTeamInput,
) -> Result<TeamExtension, CreateTeamError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(CreateTeamError::NoDatabase)?;

    input.validate().map_err(CreateTeamError::ValidationError)?;

    client
        .create_team(input)
        .await
        .map_err(|e| CreateTeamError::DatabaseError(e.to_string()))
}

#[tauri::command]
pub(crate) async fn delete_team(app: AppHandle, id: i32) -> Result<(), DeleteTeamError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(DeleteTeamError::NoDatabase)?;

    let deletion = client.delete_team(id).await;

    match deletion {
        Ok(deletion) => {
            if deletion.rows_affected == 1 {
                Ok(())
            } else {
                Err(DeleteTeamError::NotFound(id))
            }
        }
        Err(e) => Err(DeleteTeamError::DatabaseError(e.to_string())),
    }
}

#[tauri::command]
pub(crate) async fn db_migrate_up_down(app: AppHandle) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("could not access the database".to_owned())?;

    client.refresh().await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn get_groups(app: AppHandle) -> Result<Vec<db::team_group::Model>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client.get_groups().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn create_group(
    app: AppHandle,
    tag: String,
) -> Result<db::team_group::Model, CreateGroupError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(CreateGroupError::NoDatabase)?;

    client.create_group(tag).await
}

#[tauri::command]
pub(crate) async fn delete_group(app: AppHandle, id: i32) -> Result<(), DeleteGroupError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(DeleteGroupError::NoDatabase)?;

    match client.delete_group(id).await {
        Ok(result) if result.rows_affected == 1 => Ok(()),
        Ok(..) => Err(DeleteGroupError::NotFound(id)),
        Err(e) => Err(DeleteGroupError::DatabaseError(e.to_string())),
    }
}

#[tauri::command]
pub(crate) async fn get_time_slots(
    app: AppHandle,
    field_id: i32,
) -> Result<Vec<TimeSlotExtension>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_time_slots(field_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn create_time_slot(
    app: AppHandle,
    input: CreateTimeSlotInput,
) -> Result<TimeSlotExtension, TimeSlotError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(TimeSlotError::NoDatabase)?;

    client.create_time_slot(input).await
}

#[tauri::command]
pub(crate) async fn move_time_slot(
    app: AppHandle,
    input: MoveTimeSlotInput,
) -> Result<(), TimeSlotError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(TimeSlotError::NoDatabase)?;

    client.move_time_slot(input).await
}

#[tauri::command]
pub(crate) async fn delete_time_slot(app: AppHandle, id: i32) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client.delete_time_slot(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_reservations_between(
    app: AppHandle,
    input: ListReservationsBetweenInput,
) -> Result<Vec<TimeSlotExtension>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .list_reservations_between(input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn load_all_teams(app: AppHandle) -> Result<Vec<TeamExtension>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client.load_all_teams().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn update_region(
    app: AppHandle,
    input: EditRegionInput,
) -> Result<db::region::Model, EditRegionError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(EditRegionError::NoDatabase)?;

    client.edit_region(input).await
}

#[tauri::command]
pub(crate) async fn update_team(
    app: AppHandle,
    input: EditTeamInput,
) -> Result<TeamExtension, EditTeamError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(EditTeamError::NoDatabase)?;

    client.edit_team(input).await
}

#[tauri::command]
pub(crate) async fn get_targets(app: AppHandle) -> Result<Vec<TargetExtension>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_targets()
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn create_target(app: AppHandle) -> Result<TargetExtension, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .create_target()
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn delete_target(app: AppHandle, id: i32) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .delete_target(id)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn target_add_group(
    app: AppHandle,
    target_id: i32,
    group_id: i32,
) -> Result<TargetExtension, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .target_group_op(target_id, group_id, db::TargetOp::Insert)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn target_delete_group(
    app: AppHandle,
    target_id: i32,
    group_id: i32,
) -> Result<TargetExtension, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .target_group_op(target_id, group_id, db::TargetOp::Delete)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn generate_pre_schedule_report(
    app: AppHandle,
    input: PreScheduleReportInput,
) -> Result<PreScheduleReport, PreScheduleReportError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(PreScheduleReportError::NoDatabase)?;

    client.generate_pre_schedule_report(input).await
}

#[tauri::command]
pub(crate) async fn get_reservation_types(
    app: AppHandle,
) -> Result<Vec<db::reservation_type::Model>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_reservation_types()
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn create_reservation_type(
    app: AppHandle,
    input: CreateReservationTypeInput,
) -> Result<db::reservation_type::Model, CreateReservationTypeError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CreateReservationTypeError::NoDatabase)?;

    client.create_reservation_type(input).await
}

#[tauri::command]
pub(crate) async fn delete_reservation_type(app: AppHandle, id: i32) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .delete_reservation_type(id)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn update_reservation_type(
    app: AppHandle,
    reservation_type: db::reservation_type::Model,
) -> Result<(), CreateReservationTypeError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CreateReservationTypeError::NoDatabase)?;

    client.edit_reservation_type(reservation_type).await
}

#[tauri::command]
pub(crate) async fn get_supported_concurrency_for_field(
    app: AppHandle,
    input: FieldSupportedConcurrencyInput,
) -> Result<Vec<FieldConcurrency>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_supported_concurrency_for_field(input)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn update_reservation_type_concurrency_for_field(
    app: AppHandle,
    input: UpdateReservationTypeConcurrencyForFieldInput,
) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .update_reservation_type_concurrency_for_field(input)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn get_non_default_reservation_type_concurrency_associations(
    app: AppHandle,
) -> Result<Vec<FieldConcurrency>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_non_default_reservation_type_concurrency_associations()
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn update_target_reservation_type(
    app: AppHandle,
    input: UpdateTargetReservationTypeInput,
) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .update_target_reservation_type(input)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn generate_schedule_payload(
    app: AppHandle,
) -> Result<
    Vec<ScheduledInput<TeamExtension, TeamCollection, FieldExtension>>,
    GetScheduledInputsError,
> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(GetScheduledInputsError::NoDatabase)?;

    client.get_scheduled_inputs().await
}

#[tauri::command]
pub(crate) async fn schedule(
    app: AppHandle,
    authorization_token: String,
) -> Result<db::schedule::Model, ScheduleRequestError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(ScheduleRequestError::NoDatabase)?;

    let input = client
        .get_scheduled_inputs()
        .await
        .map_err(|e| ScheduleRequestError::DatabaseError(e.to_string()))?;

    let scheduled_output = send_grpc_schedule_request(&input, authorization_token).await?;

    // force error conversion with `?`
    let model = client.save_schedule(scheduled_output).await?;
    Ok(model)
}

#[tauri::command]
pub(crate) async fn get_schedules(app: AppHandle) -> Result<Vec<db::schedule::Model>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_schedules()
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn delete_schedule(app: AppHandle, id: i32) -> Result<(), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .delete_schedule(id)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn update_schedule(
    app: AppHandle,
    input: EditScheduleInput,
) -> Result<db::schedule::Model, EditScheduleError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(EditScheduleError::NoDatabase)?;

    client.edit_schedule(input).await
}

#[tauri::command]
pub(crate) async fn get_schedule(
    app: AppHandle,
    id: i32,
) -> Result<db::schedule::Model, LoadScheduleError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(LoadScheduleError::NoDatabase)?;

    client.get_schedule(id).await
}

#[tauri::command]
pub(crate) async fn health_probe() -> Result<ServerHealth, HealthProbeError> {
    net::health_probe().await
}

#[tauri::command]
pub(crate) async fn get_schedule_games(
    app: AppHandle,
    schedule_id: i32,
) -> Result<(db::schedule::Model, Vec<db::schedule_game::Model>), String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or("database was not initialized".to_owned())?;

    client
        .get_schedule_games(schedule_id)
        .await
        .map_err(|e| format!("{}:{} {e}", file!(), line!()))
}

#[tauri::command]
pub(crate) async fn get_team(app: AppHandle, id: i32) -> Result<TeamExtension, LoadTeamsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadTeamsError::NoDatabase)?;

    client.get_team(id).await
}

#[tauri::command]
pub(crate) fn get_scheduler_url() -> String {
    net::get_scheduler_url()
}

#[tauri::command]
pub(crate) fn generate_code_challenge() -> (String, String) {
    let entropy = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(128)
        .collect::<Vec<_>>();
    let mut hasher = Sha256::new();
    hasher.update(&entropy);
    let plain = String::from_utf8(entropy).unwrap();
    // Google OAUTH requires no padding
    let sha256 = base64::prelude::BASE64_URL_SAFE
        .encode(hasher.finalize())
        .trim_end_matches('=')
        .to_owned();

    (plain, sha256)
}

#[tauri::command]
pub(crate) async fn get_github_access_token(
    code: String,
) -> Result<OAuthAccessTokenExchange, String> {
    net::get_github_access_token(code)
        .await
        .map_err(|e| e.to_string())
}
