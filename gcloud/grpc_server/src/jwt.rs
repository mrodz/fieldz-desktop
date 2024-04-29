use jwt_simple::prelude::*;
use pem::Pem;
use std::collections::HashMap;
use thiserror::Error;
use x509_certificate::X509Certificate;

pub const SECURE_TOKEN_ENDPOINT: &str =
    "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";

pub const FIELDZ_JWT_AUDIENCE: &str = "fieldmasterapp";
pub const FIELDZ_JWT_ISSUER: &str = "https://securetoken.google.com/fieldmasterapp";

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
    #[error(transparent)]
    X509Error(#[from] x509_certificate::X509CertificateError),
    #[error(transparent)]
    Pem(#[from] pem::PemError),
    #[error("The date issued for the JWT is not in the past")]
    JWTDateIssued,
    #[error("This JWT is expired")]
    JWTExpired,
    #[error("This JWT has the wrong audience")]
    JWTAudience,
    #[error("This JWT has the wrong issuer")]
    JWTIssuer,
    #[error("This JWT verifies a user that was not authenticated in the past")]
    JWTCustomAuthTime,
    #[error("This JWT is missing its subject claim, or it is an empty string")]
    JWTSubject,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomFirebaseClaims {
    auth_time: Option<u64>,
}

pub async fn active_google_signing_keys(
) -> Result<HashMap<String, String>, FirebaseJWTValidationError> {
    let response = reqwest::get(SECURE_TOKEN_ENDPOINT)
        .await
        .map_err(|e| FirebaseJWTValidationError::NetErrFetchGoogleAPI(dbg!(e).to_string()))?;

    let mut key_to_cert: HashMap<String, String> = response
        .json()
        .await
        .map_err(|e| FirebaseJWTValidationError::JsonParseError(dbg!(e).to_string()))?;

    /*
     * The JSON parser will pick up an extra newline character, which isn't
     * valid in the context of a x509 certificate. The last tokens should be
     * `----- END CERTIFICATE -----` without a newline.
     */
    for cert in key_to_cert.values_mut() {
        if cert.ends_with('\n') {
            cert.pop();
        }
    }

    Ok(key_to_cert)
}

pub fn validate_jwt_metadata(
    token: &str,
    signing_keys: &HashMap<String, String>,
) -> Result<String, FirebaseJWTValidationError> {
    let metadata = Token::decode_metadata(token)?;

    let Some(key_id) = metadata.key_id() else {
        return Err(FirebaseJWTValidationError::JWTNoKID);
    };

    let algorithm = metadata.algorithm();

    if algorithm != "RS256" {
        return Err(FirebaseJWTValidationError::JWTIncorrectAlgorithm(
            algorithm.to_owned(),
        ));
    }

    signing_keys
        .get(key_id)
        .cloned()
        .ok_or(FirebaseJWTValidationError::JWTIncorrectKID)
}

pub fn verify_jwt_token(
    token: &str,
    certificate: X509Certificate,
) -> Result<JWTClaims<CustomFirebaseClaims>, FirebaseJWTValidationError> {
    let public_key = certificate.public_key_data();
    let pem = Pem::new("RSA PUBLIC KEY", public_key);
    let public_pkcs1_pem = pem::encode(&pem);
    let public_jwt_key = RS256PublicKey::from_pem(&public_pkcs1_pem)?;
    public_jwt_key
        .verify_token::<CustomFirebaseClaims>(token, None)
        .map_err(FirebaseJWTValidationError::JWTError)
}

pub fn verify_jwt_claims(
    jwt_claims: &JWTClaims<CustomFirebaseClaims>,
) -> Result<String, FirebaseJWTValidationError> {
    if !jwt_claims
        .issued_at
        .is_some_and(|issued_at| issued_at < Clock::now_since_epoch())
    {
        return Err(FirebaseJWTValidationError::JWTDateIssued);
    }

    if !jwt_claims
        .expires_at
        .is_some_and(|expires_at| expires_at > Clock::now_since_epoch())
    {
        return Err(FirebaseJWTValidationError::JWTExpired);
    }

    if !jwt_claims
        .audiences
        .as_ref()
        .is_some_and(|audiences| audiences.contains(&HashSet::from_strings(&[FIELDZ_JWT_AUDIENCE])))
    {
        return Err(FirebaseJWTValidationError::JWTAudience);
    }

    if !jwt_claims
        .issuer
        .as_ref()
        .is_some_and(|issuer| issuer == FIELDZ_JWT_ISSUER)
    {
        return Err(FirebaseJWTValidationError::JWTIssuer);
    }

    if !jwt_claims
        .custom
        .auth_time
        .is_some_and(|auth_time| auth_time < Clock::now_since_epoch().as_ticks())
    {
        return Err(FirebaseJWTValidationError::JWTCustomAuthTime);
    }

    if let Some(ref subject) = jwt_claims.subject {
        if !subject.is_empty() {
            return Ok(subject.clone());
        }
    }

    Err(FirebaseJWTValidationError::JWTSubject)
}

/// https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library
pub async fn validate_jwt(token: &str) -> Result<String, FirebaseJWTValidationError> {
    let signing_keys = active_google_signing_keys().await?;

    let cert = validate_jwt_metadata(token, &signing_keys)?;
    let x509_cert = X509Certificate::from_pem(cert.as_bytes())?;

    let verification_result = verify_jwt_token(token, x509_cert)?;

    verify_jwt_claims(&verification_result)
}
