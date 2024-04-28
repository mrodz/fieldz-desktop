use serde::Deserialize;
use std::{collections::HashMap, fmt::Display};
use thiserror::Error;

use jwt_simple::prelude::*;

pub const SECURE_TOKEN_ENDPOINT: &str =
    "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";

pub async fn active_google_signing_keys(
) -> Result<HashMap<String, String>, FirebaseJWTValidationError> {
    let response = reqwest::get(SECURE_TOKEN_ENDPOINT)
        .await
        .map_err(|e| FirebaseJWTValidationError::NetErrFetchGoogleAPI(dbg!(e).to_string()))?;

    let key_to_cert: HashMap<String, String> = response
        .json()
        .await
        .map_err(|e| FirebaseJWTValidationError::JsonParseError(dbg!(e).to_string()))?;

    Ok(key_to_cert)
}

#[derive(Error, Debug)]
pub enum FirebaseJWTValidationError {
    #[error("Could not fetch latest secure token: {0}")]
    NetErrFetchGoogleAPI(String),
    #[error("Could not parse JSON object: {0}")]
    JsonParseError(String),
    #[error("Missing `kid` in JWT")]
    JWTNoKID,
    #[error("Expected RS256 Algorithm, got {0}")]
    JWTIncorrectAlgorithm(String),
	#[error("KID not present on API")]
	JWTIncorrectKID,
	#[error(transparent)]
	JWTError(#[from] jwt_simple::Error),
}

/// https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library
pub async fn validate_jwt(token: impl AsRef<str>) -> Result<(), FirebaseJWTValidationError> {
    let token = token.as_ref();

    let metadata = Token::decode_metadata(&token)?;

    let Some(key_id) = metadata.key_id() else {
        return Err(FirebaseJWTValidationError::JWTNoKID);
    };

    let algorithm = metadata.algorithm();

    if algorithm != "RS256" {
        return Err(FirebaseJWTValidationError::JWTIncorrectAlgorithm(
            algorithm.to_owned(),
        ));
    }

	let mut signing_keys = active_google_signing_keys().await?;

	let Some(cert) = signing_keys.get_mut(key_id) else {
		return Err(FirebaseJWTValidationError::JWTIncorrectKID);
	};

	if cert.ends_with('\n') {
		cert.pop();
	}

	/*
	 * Passed stage 1 verification
	 */

	let public_key = dbg!(RS256PublicKey::from_pem(dbg!(cert)))?;

	let verification_result = public_key.verify_token::<NoCustomClaims>(token, None)?;

	dbg!(verification_result);

    todo!()
}
