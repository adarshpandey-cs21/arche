<div align="center">

# arche

**The opinionated backend foundation for Axum applications.**

[![Crates.io](https://img.shields.io/crates/v/arche.svg)](https://crates.io/crates/arche)
[![Documentation](https://img.shields.io/docsrs/arche)](https://docs.rs/arche)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Cloud integrations, databases, auth, LLM inference, encryption, streaming JSON/CSV,
WebSockets, and structured error handling — wired up and ready to go.

`arche` sits *around* Axum, not in place of it.

[Getting Started](#getting-started) · [Modules](#modules) · [API Reference](#api-reference) · [Design Principles](#design-principles)

</div>

---

## Why arche?

Every backend service re-implements the same infrastructure plumbing — cloud SDK
setup, database pools, auth primitives, error handling, config resolution. **arche**
bundles these into a single, cohesive Rust crate built on well-established libraries
so you can skip the boilerplate and focus on business logic.

## Getting Started

Add arche to your `Cargo.toml`:

```toml
[dependencies]
arche = "2.3.0"
```

## Modules

| Module | What it does |
|---|---|
| [`aws`](#aws) | S3, SES, and KMS via official AWS SDKs |
| [`gcp`](#gcp) | Google Drive, Sheets, and **Vertex AI** (Gemini + Claude) |
| [`database`](#database) | Postgres and Redis connection pooling with health checks |
| [`jwt`](#jwt) | HS256 token generation, verification, and expiry helpers |
| [`csv`](#csv) | Async CSV read/write — batch, streaming, and from URL |
| [`json`](#json) | Streaming JSON array parsing with metadata extraction |
| [`crypto`](#crypto) | AES-128-CBC encryption with PBKDF2 key derivation |
| [`sockets`](#sockets) | WebSocket connection registry with broadcast |
| [`error`](#error) | Axum-compatible structured error responses (400–503) |
| [`utils`](#utils) | Timestamp validation, date/time conversions, pagination |

Every service module exports a **config builder** so you can wire up credentials
programmatically — or omit it entirely and let arche resolve everything from
environment variables.

```rust
// Pass None to resolve entirely from env vars
let pool = arche::database::pg::get_pg_pool(None).await?;

// Or configure explicitly
let config = arche::database::pg::PgConfigBuilder::default()
    .host(Some("localhost".into()))
    .port(Some(5432))
    .build();
let pool = arche::database::pg::get_pg_pool(config).await?;
```

All components are modular and explicit — nothing is hidden or magical.

---

## API Reference

### AWS

AWS SDK integrations built on official SDKs. Default region: `ap-south-1`.

#### S3

```rust
use arche::aws::s3::{get_s3_client, S3ConfigBuilder};

// From env vars
let client = get_s3_client(None).await?;

// Or with explicit config
let config = S3ConfigBuilder::default()
    .credential_source(Some("env".into()))
    .access_key_id(Some("AKIA...".into()))
    .secret_access_key(Some("secret".into()))
    .build();
let client = get_s3_client(config).await?;
```

| Env Var | Description |
|---|---|
| `S3_CRED_SOURCE` | `"IAM"` (default) or `"env"` |
| `S3_ACCESS_KEY_ID` | Required when source is `"env"` |
| `S3_SECRET_ACCESS_KEY` | Required when source is `"env"` |
| `S3_REGION` | AWS region (default: `ap-south-1`) |

#### KMS

```rust
use arche::aws::kms::KMSClient;

// Default region
let kms = KMSClient::new_with_region("ap-south-1").await;

// Encrypt / decrypt
let ciphertext = kms.encrypt("alias/my-key", b"sensitive data").await?;
let plaintext = kms.decrypt(&ciphertext).await?;

// Decrypt base64-encoded ciphertext directly
let plaintext = kms.decrypt_base64("base64string...").await?;
```

| Env Var | Description |
|---|---|
| `AWS_REGION` | AWS region (default: `ap-south-1`) |

#### SES

```rust
use arche::aws::ses::SESClient;

let ses = SESClient::new_with_region("ap-south-1").await;

// Plain email (with optional HTML body)
let message_id = ses.send_email(
    "from@example.com",
    "to@example.com",
    "Subject line",
    "Plain text body",
    Some("<h1>HTML body</h1>"),
).await?;

// Templated email
let message_id = ses.send_templated_email(
    "from@example.com",
    "to@example.com",
    "TemplateName",
    r#"{"name": "Alice"}"#,
).await?;
```

| Env Var | Description |
|---|---|
| `AWS_REGION` | AWS region (default: `ap-south-1`) |

---

### GCP

Google Cloud Platform integrations using service account authentication.

#### Drive

```rust
use arche::gcp::drive::{get_drive_client, GcpDriveConfigBuilder};

let drive = get_drive_client(None).await?;
```

| Env Var | Description |
|---|---|
| `GCP_DRIVE_KEY` | Path to service account JSON key file |

#### Sheets

```rust
use arche::gcp::sheets::{get_sheets_client, GcpSheetsConfigBuilder};

let sheets = get_sheets_client(None).await?;
```

| Env Var | Description |
|---|---|
| `GCP_SHEETS_KEY` | Path to service account JSON key file |

#### Vertex AI

Unified LLM client for **Gemini** and **Anthropic Claude** models on Google Cloud.
Supports both streaming and non-streaming inference, function calling, and flexible
authentication (API key for Gemini, service account for both providers).

```rust
use arche::gcp::vertex::{
    get_vertex_client, VertexConfig,
    GenerateRequest, Message, Provider, StreamChunk,
};

// From env vars
let client = get_vertex_client(None).await?;

// Or with explicit config
let client = get_vertex_client(
    VertexConfig::default().with_api_key("your-api-key")
).await?;

let request = GenerateRequest::new(
    Provider::Gemini,
    "gemini-2.0-flash",
    vec![Message::user("Explain quantum computing in one sentence.")],
)
.with_system("You are a helpful assistant.")
.with_max_tokens(256)
.with_temperature(0.7);

// Non-streaming
let response = client.generate(request).await?;
println!("{}", response.text().unwrap());
println!("Tokens: {:?}", response.usage);

// Streaming
use futures::StreamExt;

let mut stream = client.stream_generate(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk? {
        StreamChunk::Text(text) => print!("{text}"),
        StreamChunk::Done { finish_reason } => println!("\n[{finish_reason}]"),
    }
}
```

**Function calling:**

```rust
use arche::gcp::vertex::ToolDefinition;

let tools = vec![ToolDefinition {
    name: "get_weather".into(),
    description: "Get current weather for a city".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    }),
}];

let request = GenerateRequest::new(
    Provider::Gemini,
    "gemini-2.0-flash",
    vec![Message::user("What's the weather in Tokyo?")],
)
.with_tools(tools);

let response = client.generate(request).await?;
for call in response.tool_calls() {
    // Handle tool calls
}
```

**Using Claude on Vertex AI** (requires service account auth):

```rust
let request = GenerateRequest::new(
    Provider::Anthropic,
    "claude-sonnet-4-20250514",
    vec![Message::user("Hello, Claude!")],
)
.with_max_tokens(1024);

let response = client.generate(request).await?;
```

**Authentication:**

| Method | When | Env Vars |
|---|---|---|
| API Key | Gemini only | `VERTEX_API_KEY` or `GEMINI_API_KEY` |
| Service Account | Gemini + Anthropic | `VERTEX_PROJECT_ID`, `VERTEX_REGION`, `GOOGLE_APPLICATION_CREDENTIALS` |

If an API key is present, it takes priority. Service account auth is required for
Anthropic models. Default region: `asia-south1`.

---

### Database

#### Postgres

Connection pooling with `sqlx`, configurable credentials, and health checks.

```rust
use arche::database::pg::{get_pg_pool, test_pg, PgConfigBuilder};

let pool = get_pg_pool(None).await?;
let is_healthy = test_pg(pool.clone()).await?;
```

| Env Var | Description |
|---|---|
| `PG_HOST` | Database host |
| `PG_PORT` | Database port |
| `PG_DATABASE` | Database name |
| `PG_MAX_CONN` | Maximum pool connections |
| `PG_USERNAME` | Username |
| `PG_PASSWORD` | Password |
| `PG_CREDENTIALS` | JSON string `{"username":"...","password":"..."}` (alternative to separate vars) |

#### Redis

Connection pooling with `bb8`, optional password auth, and health checks.

```rust
use arche::database::redis::{get_redis_pool, test_redis, RedisConfigBuilder};

let pool = get_redis_pool(None).await?;
let is_healthy = test_redis(pool.clone()).await?;
```

| Env Var | Description |
|---|---|
| `REDIS_HOST` | Redis host |
| `REDIS_PORT` | Redis port |
| `REDIS_MAX_CONN` | Maximum pool connections |
| `REDIS_PASSWORD` | Optional password |

---

### JWT

Token generation and verification using HS256.

```rust
use arche::jwt::{generate_tokens, verify_token, generate_expiry_time};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

// Generate an access + refresh token pair
let tokens = generate_tokens(
    Claims { sub: "user_123".into(), exp: generate_expiry_time(3600) },
    Claims { sub: "user_123".into(), exp: generate_expiry_time(86400) },
    &access_secret,
    &refresh_secret,
)?;

// Verify a token
let data = verify_token::<Claims>(&tokens.access_token, &access_secret, None)?;
```

---

### CSV

Async CSV processing powered by `csv-async`. Supports reading from bytes, files, and
URLs — with both batch and streaming modes.

```rust
use arche::csv::CsvClient;

// Default config (comma-delimited, with headers)
let csv = CsvClient::new();

// Or customize
let csv = CsvClient::new()
    .delimiter(b';')
    .has_headers(true)
    .flexible(true);
```

#### Reading

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Record { name: String, age: u32, city: String }

// From bytes
let records: Vec<Record> = csv.read().from_bytes(data).deserialize().await?;

// From file
let records: Vec<Record> = csv.read().from_file("data.csv").deserialize().await?;

// From URL
let records: Vec<Record> = csv.read().from_url("https://example.com/data.csv")
    .deserialize().await?;

// Batch processing (memory-efficient for large files)
csv.read().from_file("large.csv")
    .deserialize_batched(1000, |batch: Vec<Record>| async move {
        // Process 1000 records at a time
        Ok(())
    }).await?;
```

#### Writing

```rust
use serde::Serialize;

#[derive(Serialize)]
struct Output { name: String, score: f64 }

let records = vec![
    Output { name: "Alice".into(), score: 95.5 },
    Output { name: "Bob".into(),   score: 87.0 },
];

// To bytes
let bytes: Vec<u8> = csv.write_all(&records).await?;

// To file
csv.write_file("output.csv", &records).await?;
```

#### Streaming

```rust
// Record-by-record reading
let mut stream = csv.read().from_file("large.csv").stream().await?;
while let Some(record) = stream.next_deserialized::<Record>().await {
    let record = record?;
}

// Record-by-record writing
let mut writer = csv.writer_to_file("output.csv").await?;
writer.serialize(&Output { name: "Alice".into(), score: 95.5 }).await?;
writer.finish().await?;
```

---

### JSON

Streaming JSON array parsing optimized for large payloads. Extracts metadata fields
before the target array and streams array elements one-by-one or in batches —
without loading the full document into memory.

```rust
use arche::json::JsonClient;
use serde::Deserialize;

#[derive(Deserialize)]
struct Item { id: u64, name: String }

let json = JsonClient::new();

// Stream a root-level JSON array from bytes
let source = json.from_bytes(data);
let mut stream = source.stream_root_array();

while let Some(item) = stream.next::<Item>().await {
    let item = item?;
}

// Stream a nested array with metadata capture
// Given: {"total": 1000, "items": [{...}, {...}, ...]}
let json = JsonClient::new();
let source = json.from_bytes(data);
let mut stream = source.stream_array("items").await;

while let Some(item) = stream.next::<Item>().await {
    let item = item?;
}
let total: u64 = stream.field("total")?;

// Batch iteration
let batch = stream.next_batch::<Item>(100).await;

// Stream directly from S3
let source = JsonClient::new().from_s3(&s3_client, "my-bucket", "data.json").await?;
let mut stream = source.stream_array("results").await;
```

---

### Crypto

AES-128-CBC encryption with PBKDF2-HMAC-SHA1 key derivation (65,536 iterations).

```rust
use arche::crypto::{encrypt_cbc, decrypt_cbc};

let secret = "my-secret-key";
let salt = "my-salt-value-16"; // minimum 16 bytes

// Encrypt — returns raw ciphertext bytes
let ciphertext = encrypt_cbc(secret, salt, "sensitive data")?;

// Decrypt — expects base64-encoded ciphertext input
let plaintext = decrypt_cbc(secret, salt, &base64_ciphertext)?;
```

---

### Sockets

WebSocket connection registry with broadcast support. Manages a thread-safe map of
active connections for fan-out messaging.

```rust
use arche::sockets::SocketConnectionManager;

let manager = SocketConnectionManager::new();

// Register a connection (typically in a WebSocket upgrade handler)
manager.add(&connection_id, sender)?;

// Broadcast to all connected clients
manager.broadcast("Hello, everyone!".into())?;

// List active connections
let ids = manager.get_connections()?;

// Remove a connection on disconnect
manager.remove(connection_id)?;
```

---

### Error

Axum-compatible structured error handling. Every variant converts to a JSON response
with the appropriate HTTP status code.

```rust
use arche::error::AppError;

async fn handler() -> Result<impl axum::response::IntoResponse, AppError> {
    Err(AppError::Unauthorized)
}
```

**Variants:**

| Variant | Status | Constructor |
|---|---|---|
| `BadRequest` | 400 | `AppError::bad_request(errors, message, description)` |
| `Unauthorized` | 401 | Direct construction |
| `Forbidden` | 403 | Direct construction |
| `NotFound` | 404 | `AppError::not_found("resource")` |
| `Conflict` | 409 | `AppError::conflict("message")` |
| `UnprocessableEntity` | 422 | `AppError::unprocessable_entity(errors, message, description)` |
| `DependencyFailed` | 424 | `AppError::dependency_failed("upstream", "detail")` |
| `InternalError` | 500 | `AppError::internal_error(error, message)` |
| `Unavailable` | 503 | Direct construction |

`InternalError` responses are **sanitized by default** — no leaked SQL or infra
details. Enable `verbose-errors` to expose raw error details (dev/staging only):

```toml
arche = { version = "2.3.0", features = ["verbose-errors"] }
```

---

### Utils

Date/time conversion traits and pagination helpers.

```rust
use arche::utils::{validate_timestamp, FromOffsetDateTime, PaginationParams};
use time::OffsetDateTime;

// Check if a timestamp is in the future
let is_valid = validate_timestamp(timestamp, false)?;

// Convert OffsetDateTime to ISO string
let iso = offset_dt.to_iso_string()?;

// Pagination query params (for Axum extractors)
let params = PaginationParams { page_number: Some(1), page_size: Some(20) };
```

---

## Re-exported Dependencies

arche re-exports these crates so you don't need to add them separately:

`axum` · `tokio` · `serde` · `serde_json` · `sqlx` · `time` · `tracing` · `tracing-subscriber` · `reqwest` · `jsonwebtoken` · `nanoid` · `thiserror` · `base64` · `bb8` · `bb8-redis` · `csv-async` · `futures` · `tokio-stream` · `dotenv` · `aws-config` · `aws-sdk-s3` · `aws-sdk-sesv2` · `aws-sdk-kms` · `google-drive3` · `google-sheets4`

---

## Design Principles

- **Explicit over implicit** — no hidden global state or magic
- **Composition over inheritance** — thin wrappers you combine as needed
- **Production-first defaults** — sane defaults, sanitized errors, pooled connections
- **Async-native** — built on Tokio from the ground up

## What arche is *not*

- A framework that replaces Axum
- A code generator or project template
- A monolithic abstraction over third-party libraries

---

## License

[MIT](LICENSE)
