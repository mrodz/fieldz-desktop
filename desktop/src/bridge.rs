use db::{
    CreateFieldInput, CreateGroupError, CreateRegionInput, CreateTeamError, CreateTeamInput,
    FieldValidationError, RegionValidationError, TeamExtension,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use thiserror::Error;

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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateRegionError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("bad input")]
    ValidationError(RegionValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DeleteRegionError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum LoadRegionError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum LoadFieldsError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateFieldError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("bad input")]
    ValidationError(FieldValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DeleteFieldError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum LoadTeamsError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DeleteTeamError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DeleteGroupError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
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
