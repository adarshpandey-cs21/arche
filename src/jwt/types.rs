#[derive(serde::Deserialize, serde::Serialize, sqlx::FromRow)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}
