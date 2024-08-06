use std::borrow::Cow;
use std::path::Path;

use backend::ScheduledInput;
use base64::Engine;
use db::{errors::*, CoachConflictTeamInput, CreateCoachConflictInput, NameMax64, RegionMetadata};
use db::{
    CoachConflict, CopyTimeSlotsInput, CreateFieldInput, CreateRegionInput,
    CreateReservationTypeInput, CreateTeamInput, CreateTimeSlotInput, EditRegionInput,
    EditScheduleInput, EditTeamInput, FieldConcurrency, FieldExtension,
    FieldSupportedConcurrencyInput, ListReservationsBetweenInput, MoveTimeSlotInput,
    PreScheduleReport, PreScheduleReportInput, TargetExtension, TeamCollection, TeamExtension,
    TimeSlotExtension, UpdateReservationTypeConcurrencyForFieldInput,
    UpdateTargetReservationTypeInput, Validator,
};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::net::{
    self, send_grpc_schedule_request, HealthProbeError, OAuthAccessTokenExchange,
    ScheduleRequestError, ServerHealth, TwitterOAuthFlowStageOne, TwitterOAuthFlowStageTwo,
};
use crate::{SafeAppState, ACTIVE_PROFILE_BIN_KEY};

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
    Vec<ScheduledInput<TeamExtension, TeamCollection, FieldExtension, CoachConflict>>,
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
pub(crate) fn get_scheduler_url() -> Option<String> {
    net::try_get_scheduler_url()
        .inspect_err(|e| eprintln!("COMMAND(get_scheduler_url) {e} {}:{}", line!(), column!()))
        .ok()
        .map(Cow::into_owned)
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
    app: AppHandle,
    code: String,
) -> Result<OAuthAccessTokenExchange, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let net_client = lock
        .connection_pool
        .as_ref()
        .ok_or("network client was not initialized".to_owned())?;

    net::get_github_access_token(code, net_client)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn copy_time_slots(
    app: AppHandle,
    input: CopyTimeSlotsInput,
) -> Result<Vec<TimeSlotExtension>, CopyTimeSlotsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CopyTimeSlotsError::NoDatabase)?;

    client.copy_time_slots(input).await
}

#[tauri::command]
pub(crate) async fn delete_time_slots_batched(
    app: AppHandle,
    start_id: i32,
    end_id: i32,
) -> Result<(), DeleteTimeSlotsError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(DeleteTimeSlotsError::NoDatabase)?;
    client.delete_time_slots(start_id, end_id).await
}

#[tauri::command]
pub(crate) async fn create_coaching_conflict(
    app: AppHandle,
    input: CreateCoachConflictInput,
) -> Result<CoachConflict, CoachConflictError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CoachConflictError::NoDatabase)?;

    client.create_coaching_conflict(input).await
}

#[tauri::command]
pub(crate) async fn delete_coaching_conflict(
    app: AppHandle,
    id: i32,
) -> Result<(), CoachConflictError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CoachConflictError::NoDatabase)?;

    client.delete_coaching_conflict(id).await
}

#[tauri::command]
pub(crate) async fn coaching_conflict_team_op(
    app: AppHandle,
    input: CoachConflictTeamInput,
) -> Result<(), CoachConflictError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CoachConflictError::NoDatabase)?;

    client.coaching_conflict_team_op(input).await
}

#[tauri::command]
pub(crate) async fn coaching_conflict_rename(
    app: AppHandle,
    id: i32,
    new_name: String,
) -> Result<(), CoachConflictError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CoachConflictError::NoDatabase)?;

    client.coaching_conflict_rename(id, new_name).await
}

#[tauri::command]
pub(crate) async fn get_coach_conflicts(
    app: AppHandle,
    region_id: i32,
) -> Result<Vec<CoachConflict>, CoachConflictError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .database
        .as_ref()
        .ok_or(CoachConflictError::NoDatabase)?;

    client.get_coach_conflicts(region_id).await
}

#[tauri::command]
pub(crate) async fn get_region_metadata(
    app: AppHandle,
    region_id: i32,
) -> Result<RegionMetadata, LoadRegionError> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock.database.as_ref().ok_or(LoadRegionError::NoDatabase)?;

    client.get_region_metadata(region_id).await
}

#[tauri::command]
pub(crate) async fn begin_twitter_oauth_transaction(
    app: AppHandle,
    port: u32,
) -> Result<TwitterOAuthFlowStageOne, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .connection_pool
        .as_ref()
        .ok_or("network client was not initialized".to_owned())?;

    net::begin_twitter_oauth_transaction(port, client)
        .await
        .map_err(|e| format!("{e} {}:{}", line!(), column!()))
}

#[tauri::command]
pub(crate) async fn finish_twitter_oauth_transaction(
    app: AppHandle,
    oauth_token: String,
    oauth_token_secret: String,
    oauth_verifier: String,
) -> Result<TwitterOAuthFlowStageTwo, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let client = lock
        .connection_pool
        .as_ref()
        .ok_or("network client was not initialized".to_owned())?;

    net::finish_twitter_oauth_transaction(oauth_token, oauth_token_secret, oauth_verifier, client)
        .await
        .map_err(|e| format!("{e} {}:{}", line!(), column!()))
}

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ProfileMetadata {
    size: Option<u64>,
}

fn list_profiles_impl(profiles_directory: &Path) -> Result<Vec<(String, ProfileMetadata)>, String> {
    let paths = std::fs::read_dir(profiles_directory)
        .map_err(|e| format!("{e} {}:{}", line!(), column!()))?;

    let mut profiles = vec![];

    for entry in paths {
        let entry = entry.map_err(|e| format!("{e} {}:{}", line!(), column!()))?;

        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name() else {
            continue;
        };

        let meta = match path.join("data.sqlite").metadata() {
            Ok(metadata) => ProfileMetadata {
                size: Some(metadata.len()),
            },
            Err(_e) => ProfileMetadata::default(),
        };

        profiles.push((name.to_string_lossy().into_owned(), meta))
    }

    Ok(profiles)
}

#[tauri::command]
pub(crate) fn list_profiles(app: AppHandle) -> Result<Vec<(String, ProfileMetadata)>, String> {
    let profiles_directory = app
        .path_resolver()
        .app_data_dir()
        .ok_or("could not access app data directory".to_owned())?
        .join("profiles");

    list_profiles_impl(&profiles_directory)
}

#[tauri::command]
pub(crate) async fn get_active_profile(app: AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;
    let store = lock
        .metadata_store
        .as_ref()
        .ok_or("metadata was not initialized".to_owned())?;

    let Some(js_value) = store.get(ACTIVE_PROFILE_BIN_KEY) else {
        return Ok(None);
    };

    if js_value.is_null() {
        return Ok(None);
    }

    js_value
        .as_str()
        .ok_or("active profile was not a string".to_owned())
        .map(|str| Some(str.to_owned()))
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SelectProfileError {
    #[error("metadata was not initialized")]
    NoStore,
    #[error("could not access app data directory")]
    MissingAppData,
    #[error("there are no profiles with the name {0}")]
    MissingProfile(String),
    #[error("could not operate on store: {0}")]
    StoreError(String),
    #[error("could not swap data sources: {0}")]
    DatabaseInitError(String),
}

#[tauri::command]
pub(crate) async fn set_active_profile(
    app: AppHandle,
    name: Option<String>,
) -> Result<Option<String>, SelectProfileError> {
    let state = app.state::<SafeAppState>();
    let mut lock = state.0.lock().await;

    let profiles_directory = app
        .path_resolver()
        .app_data_dir()
        .ok_or(SelectProfileError::MissingAppData)?;

    let maybe_selected_profile_directory = if let Some(ref name) = name {
        let path = profiles_directory.join("profiles").join(name);
        if !path.exists() {
            return Err(SelectProfileError::MissingProfile(name.clone()));
        }
        path
    } else {
        profiles_directory
    };

    let db_path = format!(
        "sqlite:{}?mode=rwc",
        maybe_selected_profile_directory
            .join("data.sqlite")
            .to_string_lossy()
    );

    let db_config = db::Config::new(db_path);

    lock.database = Some(db::Client::new(&db_config).await.map_err(|e| {
        SelectProfileError::DatabaseInitError(format!("{e:?} {}:{}", line!(), column!()))
    })?);

    let active_profile_record = name
        .clone()
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null);

    println!("Reflecting changed profile: {active_profile_record:?}");

    let store = lock
        .metadata_store
        .as_mut()
        .ok_or(SelectProfileError::NoStore)?;

    store
        .insert(ACTIVE_PROFILE_BIN_KEY.to_owned(), active_profile_record)
        .map_err(|e| SelectProfileError::StoreError(e.to_string()))?;

    store
        .save()
        .map_err(|e| SelectProfileError::StoreError(e.to_string()))?;

    app.emit_all("profile-selection", &name)
        .expect("could not emit event");

    Ok(name)
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CreateProfileError {
    #[error("could not access app data directory")]
    MissingAppData,
    #[error(transparent)]
    NameTooLong(#[from] NameMax64ValidationError),
    #[error("Profile names can only contain alphanumeric characters, '_', and '-'")]
    IllegalCharacterError,
    #[error("Could not list profiles: {0}")]
    ListProfileError(String),
    #[error("A profile already exists with this name")]
    DuplicateNameError,
    #[error("This name is reserved by the file system")]
    IllegalNameError,
    #[error("IO Operation failed: {0}")]
    IOError(String),
    #[error("there are no profiles with the name {0}")]
    MissingProfile(String),
}

fn is_valid_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == ' '
}

#[tauri::command]
pub(crate) async fn create_new_profile(
    app: AppHandle,
    name: NameMax64,
) -> Result<String, CreateProfileError> {
    name.validate()?;

    let name = name
        .trim_start_matches(char::is_whitespace)
        .trim_end_matches(char::is_whitespace);

    // Reserved names in Windows
    const RESERVED_NAMES: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    if RESERVED_NAMES
        .iter()
        .any(|reserved_name| *reserved_name == name || reserved_name.to_lowercase() == name)
    {
        return Err(CreateProfileError::IllegalNameError);
    }

    if !name.chars().all(is_valid_char) {
        return Err(CreateProfileError::IllegalCharacterError);
    }

    let profiles_directory = app
        .path_resolver()
        .app_data_dir()
        .ok_or(CreateProfileError::MissingAppData)?
        .join("profiles");

    let existing_profiles =
        list_profiles_impl(&profiles_directory).map_err(CreateProfileError::ListProfileError)?;

    if existing_profiles
        .iter()
        .any(|(profile_name, _)| profile_name == name)
    {
        return Err(CreateProfileError::DuplicateNameError);
    }

    std::fs::create_dir(profiles_directory.join(name))
        .map_err(|e| CreateProfileError::IOError(e.to_string()))?;

    Ok(name.to_owned())
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DeleteProfileError {
    #[error("could not access app data directory")]
    MissingAppData,
    #[error("there are no profiles with the name {0}")]
    MissingProfile(String),
    #[error("Could not list profiles: {0}")]
    ListProfileError(String),
    #[error("metadata was not initialized")]
    NoStore,
    #[error("cannot modify the active profile")]
    ProfileIsActive,
    #[error("IO Operation failed: {0}")]
    IOError(String),
    #[error(transparent)]
    NameTooLong(#[from] NameMax64ValidationError),
    #[error("Profile names can only contain alphanumeric characters, '_', and '-'")]
    IllegalCharacterError,
}

#[tauri::command]
pub(crate) async fn delete_profile(
    app: AppHandle,
    profile_name: NameMax64,
) -> Result<(), DeleteProfileError> {
    profile_name.validate()?;

    // prevent controlling the path and routing using ".."
    if !profile_name.chars().all(is_valid_char) {
        return Err(DeleteProfileError::IllegalCharacterError);
    }

    let profiles_directory = app
        .path_resolver()
        .app_data_dir()
        .ok_or(DeleteProfileError::MissingAppData)?
        .join("profiles");

    let profile_path = profiles_directory.join(&*profile_name);

    if !profile_path.exists() {
        return Err(DeleteProfileError::MissingProfile(
            <String as ToOwned>::to_owned(&profile_name),
        ));
    }

    let state = app.state::<SafeAppState>();
    let lock = state.0.lock().await;

    if let Some(serde_json::Value::String(active_profile)) = lock
        .metadata_store
        .as_ref()
        .ok_or(DeleteProfileError::NoStore)?
        .get(ACTIVE_PROFILE_BIN_KEY)
    {
        if active_profile == &*profile_name {
            return Err(DeleteProfileError::ProfileIsActive);
        }
    }

    println!("Removing directory: {}", profile_path.to_string_lossy());

    std::fs::remove_dir_all(profile_path)
        .inspect_err(|e| eprintln!("{e:?}"))
        .map_err(|e| DeleteProfileError::IOError(e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn rename_profile(
    app: AppHandle,
    profile_name: NameMax64,
    new_name: NameMax64,
) -> Result<String, CreateProfileError> {
    new_name.validate()?;
    profile_name.validate()?;

    let new_name = new_name
        .trim_start_matches(char::is_whitespace)
        .trim_end_matches(char::is_whitespace);

    let profile_name = profile_name
        .trim_start_matches(char::is_whitespace)
        .trim_end_matches(char::is_whitespace);

    // Reserved names in Windows
    const RESERVED_NAMES: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    if RESERVED_NAMES.contains(&new_name) || RESERVED_NAMES.contains(&profile_name) {
        return Err(CreateProfileError::IllegalNameError);
    }

    if !new_name.chars().all(is_valid_char) || !profile_name.chars().all(is_valid_char) {
        return Err(CreateProfileError::IllegalCharacterError);
    }

    let profiles_directory = app
        .path_resolver()
        .app_data_dir()
        .ok_or(CreateProfileError::MissingAppData)?
        .join("profiles");

    let existing_profiles =
        list_profiles_impl(&profiles_directory).map_err(CreateProfileError::ListProfileError)?;

    if existing_profiles
        .iter()
        .any(|(profile_name, _)| profile_name == new_name)
    {
        return Err(CreateProfileError::DuplicateNameError);
    }

    let target_profile = profiles_directory.join(profile_name);

    if !target_profile.exists() {
        return Err(CreateProfileError::MissingProfile(profile_name.to_owned()));
    }

    std::fs::rename(target_profile, profiles_directory.join(new_name))
        .map_err(|e| CreateProfileError::IOError(e.to_string()))?;

    Ok(new_name.to_owned())
}
