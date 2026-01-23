use bb8::{Pool, PooledConnection};
use bb8_redis::redis::AsyncCommands;
use bb8_redis::RedisConnectionManager;

pub type RedisPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

#[allow(dead_code)]
pub async fn get_redis_pool() -> Pool<RedisConnectionManager> {
    // reading env variables for redis
    let redis_host = std::env::var("REDIS_HOST").expect("REDIS_HOST is not set");
    let redis_port = std::env::var("REDIS_PORT").expect("REDIS_PORT is not set");
    let redis_max_conn = std::env::var("REDIS_MAX_CONN").expect("REDIS_MAX_CONN is not set");

    let redis_url = format!("redis://{}:{}", redis_host, redis_port);

    let redis_conn_manager = bb8_redis::RedisConnectionManager::new(redis_url).unwrap();
    bb8::Pool::builder()
        .max_size(redis_max_conn.parse::<u32>().unwrap())
        .build(redis_conn_manager)
        .await
        .unwrap()
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
