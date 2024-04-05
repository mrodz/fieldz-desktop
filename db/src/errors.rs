use chrono::{serde::ts_milliseconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum RegionValidationError {
    #[error(transparent)]
    Name(#[from] RegionNameValidationError),
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

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RegionNameValidationError {
    #[error("region name cannot be empty")]
    EmptyName,
    #[error("region name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum FieldValidationError {
    #[error("field name cannot be empty")]
    EmptyName,
    #[error("field name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum NameMax64ValidationError {
    #[error("field name cannot be empty")]
    EmptyName,
    #[error("field name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateGroupError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this tag already exists")]
    DuplicateTag,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateTeamError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("bad input")]
    ValidationError(NameMax64ValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("the following tags do not exist: {0:?}")]
    MissingTags(Vec<String>),
    #[error("the transaction to create a team failed")]
    TransactionError,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum TimeSlotError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("this time slot is booked from {o_start} to {o_end}")]
    Overlap {
        #[serde(with = "ts_milliseconds")]
        o_start: DateTime<Utc>,
        #[serde(with = "ts_milliseconds")]
        o_end: DateTime<Utc>,
    },
    #[error("the supplied reservation type id ({0}) does not exist")]
    ReservationTypeDoesNotExist(i32),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("could not parse date: `{0}`")]
    ParseError(String),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum EditRegionError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error(transparent)]
    Name(#[from] RegionNameValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("region with id {0} not found")]
    NotFound(i32),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum EditTeamError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("bad input")]
    ValidationError(NameMax64ValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("the following tags do not exist: {0:?}")]
    MissingTags(Vec<String>),
    #[error("the transaction to create a team failed")]
    TransactionError,
    #[error("team with id {0} not found")]
    NotFound(i32),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum TargetOpError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("group with id {0} not found")]
    GroupNotFound(i32),
    #[error("team with id {0} not found")]
    TargetNotFound(i32),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("the transaction to create a team failed")]
    TransactionError,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateReservationTypeError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error(transparent)]
    Name(#[from] NameMax64ValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum PreScheduleReportError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum LoadRegionError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum LoadFieldsError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DeleteFieldError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum LoadTeamsError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DeleteGroupError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this record (id = {0}) was not found")]
    NotFound(i32),
}
