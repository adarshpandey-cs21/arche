use serde_json::Value;
use sqlx::postgres::PgPoolOptions;

use crate::config::{resolve_optional_string, resolve_required, resolve_required_string};
use crate::error::AppError;

pub use crate::config::pg::{PgConfig, PgConfigBuilder};

#[derive(sqlx::FromRow)]
pub struct PgHealth {
    pub id: String,
    pub value: String,
}

#[derive(serde::Deserialize, Debug)]
struct PgCredentials {
    username: String,
    password: String,
}

pub type PgPool = sqlx::PgPool;

fn get_credentials(config: &PgConfig) -> Result<PgCredentials, AppError> {
    let credentials_json =
        resolve_optional_string(config.credentials_json.clone(), "PG_CREDENTIALS");

    if let Some(credentials_str) = credentials_json {
        let credentials_json = serde_json::from_str::<Value>(&credentials_str).map_err(|e| {
            AppError::config_error(
                "credentials_json".to_string(),
                Some("PG_CREDENTIALS".to_string()),
                format!("Failed to parse credentials JSON: {}", e),
            )
        })?;

        let username = credentials_json
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::config_error(
                    "username".to_string(),
                    Some("PG_CREDENTIALS".to_string()),
                    "username not found in credentials JSON".to_string(),
                )
            })?
            .to_string();

        let password = credentials_json
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::config_error(
                    "password".to_string(),
                    Some("PG_CREDENTIALS".to_string()),
                    "password not found in credentials JSON".to_string(),
                )
            })?
            .to_string();

        return Ok(PgCredentials { username, password });
    }

    Ok(PgCredentials {
        username: resolve_required_string(config.username.clone(), "PG_USERNAME", "username")?,
        password: resolve_required_string(config.password.clone(), "PG_PASSWORD", "password")?,
    })
}

pub async fn get_pg_pool(config: impl Into<Option<PgConfig>>) -> Result<PgPool, AppError> {
    let config = config.into().unwrap_or_default();
    let credentials = get_credentials(&config)?;

    let host = resolve_required_string(config.host, "PG_HOST", "host")?;
    let port: u16 = resolve_required(config.port, "PG_PORT", "port")?;
    let database = resolve_required_string(config.database, "PG_DATABASE", "database")?;
    let max_conn: u32 = resolve_required(config.max_connections, "PG_MAX_CONN", "max_connections")?;

    let pg_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        credentials.username, credentials.password, host, port, database
    );

    PgPoolOptions::new()
        .max_connections(max_conn)
        .connect(&pg_url)
        .await
        .map_err(|e| {
            AppError::config_error(
                "connection".to_string(),
                None,
                format!("Failed to create PG Pool: {}", e),
            )
        })
}

pub async fn test_pg(pg_pool: sqlx::PgPool) -> bool {
    let insert_query = r#"
    INSERT INTO health (id, value) VALUES ($1, $2)"#;
    let select_query = r#"
    SELECT id, value FROM health WHERE id = $1"#;
    let delete_query = r#"
    DELETE FROM health WHERE id = $1"#;

    let id = nanoid::nanoid!();
    let value = "test-value";

    let insert_result = sqlx::query(insert_query)
        .bind(&id)
        .bind(value)
        .execute(&pg_pool)
        .await
        .expect("Failed to insert data");

    let select_result = sqlx::query_as::<_, PgHealth>(select_query)
        .bind(&id)
        .fetch_one(&pg_pool)
        .await
        .expect("Failed to select data");

    sqlx::query(delete_query)
        .bind(&id)
        .execute(&pg_pool)
        .await
        .expect("Failed to delete data");

    insert_result.rows_affected() == 1 && select_result.id == id && select_result.value == value
}
