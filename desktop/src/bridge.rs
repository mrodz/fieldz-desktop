use db::{CreateFieldInput, CreateRegionInput, FieldValidationError, RegionValidationError};
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
    #[error("database operation failed")]
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
    #[error("database operation failed")]
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
    #[error("database operation failed")]
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
    #[error("database operation failed")]
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
    #[error("database operation failed")]
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
