use serde_json::Value;
use sqlx::postgres::PgPoolOptions;

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

fn get_credentials() -> PgCredentials {
    if let Ok(credentials_str) = std::env::var("PG_CREDENTIALS") {
        let credentials_json = serde_json::from_str::<Value>(&credentials_str).unwrap();
        PgCredentials {
            username: credentials_json
                .get("username")
                .and_then(|v| v.as_str())
                .expect("username is not set")
                .to_string(),
            password: credentials_json
                .get("password")
                .and_then(|v| v.as_str())
                .expect("password is not set")
                .to_string(),
        }
    } else {
        PgCredentials {
            username: std::env::var("PG_USERNAME").expect("PG_USERNAME is not set"),
            password: std::env::var("PG_PASSWORD").expect("PG_PASSWORD is not set"),
        }
    }
}

pub async fn get_pg_pool() -> sqlx::PgPool {
    let credentials = get_credentials();

    let pg_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        credentials.username,
        credentials.password,
        std::env::var("PG_HOST").expect("PG_HOST is not set"),
        std::env::var("PG_PORT").expect("PG_PORT is not set"),
        std::env::var("PG_DATABASE").expect("PG_DATABASE is not set")
    );

    let pg_max_conn = std::env::var("PG_MAX_CONN")
        .expect("PG_MAX_CONN is not set")
        .parse::<u32>()
        .expect("PG_MAX_CONN is not a number");

    return PgPoolOptions::new()
        .max_connections(pg_max_conn)
        .connect(&pg_url)
        .await
        .expect("Failed to create PG Pool");
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
