use core::str;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum TwitterOAuthError {
    #[error(transparent)]
    Base64EncodingError(#[from] base64::EncodeSliceError),
    #[error(transparent)]
    DecodeUtf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    DeserializationError(#[from] serde::de::value::Error),
    #[error(transparent)]
    FormDataSerializationError(#[from] serde_urlencoded::ser::Error),
    #[error(transparent)]
    SigningNetworkError(#[from] reqwest::Error),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TwitterOAuthAuthorizationMetadata {
    oauth_consumer_key: String,
    oauth_nonce: String,
    oauth_timestamp: u64,
    oauth_version: String,
    oauth_token: Option<String>,
    oauth_callback: Option<Url>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum TwitterSignatureState {
    Unsigned(String),
    Signed(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TwitterOAuthSignature {
    oauth_signature: TwitterSignatureState,
    oauth_signature_method: String,
}

#[derive(Debug)]
pub struct TwitterOAuthAuthorization {
    metadata: TwitterOAuthAuthorizationMetadata,
    signature: TwitterOAuthSignature,
}

impl Display for TwitterOAuthAuthorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OAuth ")?;

        write!(
            f,
            "oauth_consumer_key=\"{}\", ",
            urlencoding::encode(&self.metadata.oauth_consumer_key)
        )?;
        write!(
            f,
            "oauth_nonce=\"{}\", ",
            urlencoding::encode(&self.metadata.oauth_nonce)
        )?;

        let Some(oauth_signature) = self.signature.get() else {
            panic!("this signature was not signed");
        };

        write!(
            f,
            "oauth_signature=\"{}\", ",
            urlencoding::encode(oauth_signature)
        )?;

        write!(
            f,
            "oauth_signature_method=\"{}\", ",
            urlencoding::encode(&self.signature.oauth_signature_method)
        )?;

        write!(f, "oauth_timestamp=\"{}\", ", self.metadata.oauth_timestamp)?;

        if let Some(ref oauth_token) = self.metadata.oauth_token {
            write!(f, "oauth_token=\"{}\", ", urlencoding::encode(oauth_token))?;
        }

        if let Some(ref oauth_callback) = self.metadata.oauth_callback {
            write!(
                f,
                "oauth_callback=\"{}\", ",
                urlencoding::encode(oauth_callback.as_str())
            )?;
        }

        write!(f, "oauth_version=\"{}\"", self.metadata.oauth_version)
    }
}

impl TwitterOAuthAuthorization {
    pub fn new_unsigned<T: Serialize + ?Sized>(
        metadata: TwitterOAuthAuthorizationMetadata,
        request_input: SigningInputPayload<T>,
    ) -> Result<Self, TwitterOAuthError> {
        let signature = TwitterOAuthSignature::new(&metadata, request_input)?;

        Ok(Self {
            metadata,
            signature,
        })
    }

    pub async fn new<'a, T: Serialize + ?Sized>(
        metadata: TwitterOAuthAuthorizationMetadata,
        request_input: SigningInputPayload<'a, T>,
        client: &reqwest::Client,
        signing_endpoint: Url,
    ) -> Result<Self, TwitterOAuthError> {
        let mut unsigned = Self::new_unsigned(metadata, request_input)?;

        unsigned.sign(client, signing_endpoint).await?;

        Ok(unsigned)
    }

    pub async fn sign(
        &mut self,
        client: &reqwest::Client,
        signing_endpoint: Url,
    ) -> Result<(), TwitterOAuthError> {
        self.signature.sign(client, signing_endpoint).await
    }
}

#[derive(Debug)]
pub struct TwitterOAuthFlow {
    oauth_consumer_key: String,
    signing_endpoint: Url,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TwitterOAuthFlowStageOne {
    oauth_token: String,
    oauth_token_secret: String,
    oauth_callback_confirmed: String,
}

impl TwitterOAuthFlow {
    pub fn new(oauth_consumer_key: impl AsRef<str>, signing_endpoint: Url) -> Self {
        Self {
            oauth_consumer_key: oauth_consumer_key.as_ref().to_owned(),
            signing_endpoint,
        }
    }

    pub async fn get_request_token(
        &self,
        oauth_callback: Url,
        client: &reqwest::Client,
    ) -> Result<TwitterOAuthFlowStageOne, TwitterOAuthError> {
        let stage_one_metadata = TwitterOAuthAuthorizationMetadata::new(
            &self.oauth_consumer_key,
            Option::<&str>::None,
            Some(oauth_callback),
        )?;

        let endpoint = Url::parse("https://api.twitter.com/oauth/request_token")?;

        let stage_one_request_input = SigningInputPayload::<[(&str, &str); 0]> {
            url: endpoint.clone(),
            form_data: &[],
            method: reqwest::Method::POST,
        };

        let stage_one_authorization = TwitterOAuthAuthorization::new(
            stage_one_metadata,
            stage_one_request_input,
            client,
            self.signing_endpoint.clone(),
        )
        .await?;

        let authorization_header = stage_one_authorization.to_string();

        let response = client
            .post(endpoint)
            .header(reqwest::header::AUTHORIZATION, authorization_header)
            .send()
            .await?;

        let parsed = serde_urlencoded::from_str(&response.text().await?)?;

        Ok(parsed)
    }
}

/// "The value for this request was generated by base64 encoding 32 bytes
/// of random data, and stripping out all non-word characters, but any
/// approach which produces a relatively random alphanumeric string should
/// be OK here.""
pub fn generate_twitter_compliant_nonce() -> Result<String, TwitterOAuthError> {
    let mut rng = rand::thread_rng();

    let nonce: [u8; 32] = std::array::from_fn(|_| rng.sample(Alphanumeric));

    // 4 * ceil(32 / 3)
    let mut output_buf = [0; 44];

    base64::prelude::BASE64_URL_SAFE.encode_slice(&nonce, &mut output_buf)?;

    Ok(str::from_utf8(&output_buf).map(str::to_owned)?)
}

pub const TWITTER_OAUTH_SIGNATURE_METHOD: &str = "HMAC-SHA1";
pub const TWITTER_OAUTH_VERSION: &str = "1.0";

impl TwitterOAuthAuthorizationMetadata {
    pub fn new(
        oauth_consumer_key: impl AsRef<str>,
        oauth_token: Option<impl AsRef<str>>,
        oauth_callback: Option<Url>,
    ) -> Result<Self, TwitterOAuthError> {
        let oauth_nonce = generate_twitter_compliant_nonce()?;

        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let oauth_timestamp = since_the_epoch.as_secs();

        Ok(Self {
            oauth_consumer_key: oauth_consumer_key.as_ref().into(),
            oauth_token: oauth_token.map(|token| token.as_ref().to_owned()),
            oauth_callback,
            oauth_nonce,
            oauth_timestamp,
            oauth_version: TWITTER_OAUTH_VERSION.into(),
        })
    }
}

struct ParametersPipeline<'a> {
    data: BTreeMap<Cow<'a, str>, String>,
}

impl<'a> ParametersPipeline<'a> {
    fn new<'b, T: Serialize + ?Sized>(
        metadata: &'a TwitterOAuthAuthorizationMetadata,
        query_pairs: BTreeMap<Cow<'b, str>, Cow<'b, str>>,
        form_data: &T,
        oauth_signature_method: impl AsRef<str>,
    ) -> Result<Self, TwitterOAuthError> {
        let form_data = serde_urlencoded::from_str::<BTreeMap<String, String>>(
            &serde_urlencoded::to_string(&form_data)?,
        )
        .expect("transitive operation should never fail");

        // The OAuth spec says to sort lexicographically, which is the default alphabetical sort for a `BTreeMap` with key `String`
        let mut result = BTreeMap::new();

        result.insert(
            Cow::Borrowed("oauth_consumer_key"),
            urlencoding::decode(&metadata.oauth_consumer_key)?.into_owned(),
        );
        result.insert(
            Cow::Borrowed("oauth_nonce"),
            urlencoding::decode(&metadata.oauth_nonce)?.into_owned(),
        );
        result.insert(
            Cow::Borrowed("oauth_signature_method"),
            urlencoding::decode(oauth_signature_method.as_ref())?.into_owned(),
        );
        result.insert(
            Cow::Borrowed("oauth_timestamp"),
            metadata.oauth_timestamp.to_string(),
        );

        result.insert(
            Cow::Borrowed("oauth_version"),
            urlencoding::decode(&metadata.oauth_version)?.into_owned(),
        );

        if let Some(ref oauth_token) = metadata.oauth_token {
            result.insert(
                Cow::Borrowed("oauth_token"),
                urlencoding::decode(&oauth_token)?.into_owned(),
            );
        }

        if let Some(ref oauth_callback) = metadata.oauth_callback {
            result.insert(
                Cow::Borrowed("oauth_callback"),
                urlencoding::decode(oauth_callback.as_str())?.into_owned(),
            );
        }

        for (param, value) in query_pairs {
            result.insert(
                Cow::Owned(param.into_owned()),
                urlencoding::decode(value.as_ref())?.into_owned(),
            );
        }

        for (form_param, form_value) in form_data {
            result.insert(
                Cow::Owned(urlencoding::decode(&form_param)?.into_owned()),
                urlencoding::decode(&form_value)?.into_owned(),
            );
        }

        let mut raw = Self {
            data: result,
        };

        raw.percent_encode_kv();

        Ok(raw)
    }

    fn percent_encode_kv(&mut self) {
        let mut encoded = BTreeMap::new();

        for (key, value) in &self.data {
            let encoded_key = urlencoding::encode(key.as_ref());
            let encoded_value = urlencoding::encode(value.as_ref());

            encoded.insert(
                Cow::Owned(encoded_key.into_owned()),
                encoded_value.into_owned(),
            );
        }

        self.data = encoded;
    }
}

impl Display for ParametersPipeline<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iterator = self.data.iter();

        let Some((first_encoded_key, first_encoded_value)) = iterator.next() else {
            return Ok(());
        };

        write!(f, "{first_encoded_key}={first_encoded_value}")?;

        for (encoded_key, encoded_value) in iterator {
            write!(f, "&{encoded_key}={encoded_value}")?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Hash, PartialOrd)]
struct SigningResponse {
    signed: String,
}

pub struct SigningInputPayload<'a, T>
where
    T: Serialize + ?Sized,
{
    url: Url,
    method: reqwest::Method,
    form_data: &'a T,
}

impl TwitterOAuthSignature {
    pub fn new<T: Serialize + ?Sized>(
        metadata: &TwitterOAuthAuthorizationMetadata,
        input: SigningInputPayload<T>,
    ) -> Result<Self, TwitterOAuthError> {
        let http_verb = input.method.as_str().to_uppercase();

        let query = input.url.query_pairs().collect::<BTreeMap<_, _>>();

        let mut base_url = input.url.clone();

        base_url.set_query(None);
        base_url.set_fragment(None);

        let base_url_percent_encoded = urlencoding::encode(base_url.as_str());

        let parameters = ParametersPipeline::new(
            metadata,
            query,
            input.form_data,
            TWITTER_OAUTH_SIGNATURE_METHOD,
        )?;

        let parameter_string = parameters.to_string();

        /*
         * Double encoding is NOT a bug and is required by the spec
         * ie: "word=hello%20world" -> "word=hello%2520world"
         */
        let parameter_string_percent_encoded = urlencoding::encode(&parameter_string);

        let unsigned_payload =
            format!("{http_verb}&{base_url_percent_encoded}&{parameter_string_percent_encoded}");

        Ok(Self {
            oauth_signature: TwitterSignatureState::Unsigned(unsigned_payload),
            oauth_signature_method: TWITTER_OAUTH_SIGNATURE_METHOD.to_owned(),
        })
    }

    pub async fn sign(
        &mut self,
        client: &reqwest::Client,
        signing_endpoint: Url,
    ) -> Result<(), TwitterOAuthError> {
        let TwitterSignatureState::Unsigned(ref raw) = self.oauth_signature else {
            return Ok(());
        };

        let response = client
            .post(signing_endpoint)
            .body(raw.clone())
            .send()
            .await?;

        let body = response.json::<SigningResponse>().await?;

        self.oauth_signature = TwitterSignatureState::Signed(body.signed);

        Ok(())
    }

    pub fn get(&self) -> Option<&str> {
        match self.oauth_signature {
            TwitterSignatureState::Unsigned(_) => None,
            TwitterSignatureState::Signed(ref str) => Some(str.as_str()),
        }
    }
}
