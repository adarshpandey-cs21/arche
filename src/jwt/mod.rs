use std::ops::Add;
use std::time::{Duration, SystemTime};

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
    errors::Error,
};

mod types;
use serde::Serialize;
use time::OffsetDateTime;
pub use types::AuthTokens;

pub fn _peek_token_data<T: serde::de::DeserializeOwned>(
    token: &str,
) -> Result<TokenData<T>, Error> {
    jsonwebtoken::dangerous::insecure_decode::<T>(token)
}

pub fn verify_token<T: serde::de::DeserializeOwned>(
    token: &str,
    key: String,
    audience: Option<String>,
) -> Result<TokenData<T>, Error> {
    let mut validation = Validation::new(Algorithm::HS256);

    match audience {
        Some(aud) => {
            validation.set_audience(&[aud]);
        }
        None => {
            validation.validate_aud = false;
        }
    }

    let secret = DecodingKey::from_secret(key.as_ref());
    decode::<T>(token, &secret, &validation)
}

pub fn generate_expiry_time(seconds: u64) -> usize {
    let duration = Duration::from_secs(seconds);
    let expiry_time = SystemTime::now().add(duration);
    OffsetDateTime::from(expiry_time).unix_timestamp() as usize
}
pub fn generate_tokens<AccessTokenData: Serialize, RefreshTokenData: Serialize>(
    access_token_data: AccessTokenData,
    refresh_token_data: RefreshTokenData,
    access_secret: &String,
    refresh_secret: &String,
) -> types::AuthTokens {
    let access_token = generate_token(access_token_data, access_secret);
    let refresh_token = generate_token(refresh_token_data, refresh_secret);

    AuthTokens {
        access_token,
        refresh_token,
    }
}

pub fn generate_token<TokenData: Serialize>(data: TokenData, secret: &String) -> String {
    encode_token(&data, secret)
}

fn encode_token<T: Serialize>(data: &T, secret: &String) -> String {
    encode(
        &Header::default(),
        data,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to generate token")
}
