use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use bb8_redis::redis::AsyncCommands;

use crate::config::{resolve_optional_string, resolve_required, resolve_required_string};
use crate::error::AppError;

pub use crate::config::redis::{RedisConfig, RedisConfigBuilder};

pub type RedisPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

#[allow(dead_code)]
pub async fn get_redis_pool(
    config: impl Into<Option<RedisConfig>>,
) -> Result<Pool<RedisConnectionManager>, AppError> {
    let config = config.into().unwrap_or_default();

    let host = resolve_required_string(config.host, "REDIS_HOST", "host")?;
    let port: u16 = resolve_required(config.port, "REDIS_PORT", "port")?;
    let max_conn: u32 =
        resolve_required(config.max_connections, "REDIS_MAX_CONN", "max_connections")?;
    let password = resolve_optional_string(config.password, "REDIS_PASSWORD");

    let redis_url = if let Some(pwd) = password {
        format!("redis://:{}@{}:{}", pwd, host, port)
    } else {
        format!("redis://{}:{}", host, port)
    };

    let redis_conn_manager = bb8_redis::RedisConnectionManager::new(redis_url).map_err(|e| {
        AppError::config_error(
            "connection_manager".to_string(),
            None,
            format!("Failed to create Redis connection manager: {}", e),
        )
    })?;

    bb8::Pool::builder()
        .max_size(max_conn)
        .build(redis_conn_manager)
        .await
        .map_err(|e| {
            AppError::config_error(
                "pool".to_string(),
                None,
                format!("Failed to create Redis pool: {}", e),
            )
        })
}

pub async fn _get_redis_conn(
    redis_pool: &Pool<RedisConnectionManager>,
) -> PooledConnection<'_, RedisConnectionManager> {
    redis_pool.get().await.unwrap()
}

pub async fn test_redis(redis_pool: bb8::Pool<bb8_redis::RedisConnectionManager>) -> bool {
    let mut redis_conn: PooledConnection<RedisConnectionManager> = redis_pool.get().await.unwrap();
    redis_conn
        .set::<&str, &str, ()>("test-key", "test-value")
        .await
        .unwrap();
    let value: String = redis_conn.get("test-key").await.unwrap();
    let _: () = redis_conn.del("test-key").await.unwrap();
    value == "test-value"
}
