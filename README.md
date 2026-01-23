# arche

**arche** is an opinionated backend foundation crate for building production-ready
applications with **Axum**.

It provides a curated set of building blocks commonly required in modern backend
services—cloud integrations, databases, authentication, middleware, and logging—
so you can focus on business logic instead of repetitive infrastructure wiring.

`arche` is designed to *sit around Axum*, not replace it.

## Why arche?

Most backend services end up re-implementing the same infrastructure concerns:

- Cloud SDK setup and ergonomics
- Database connection management
- Authentication primitives
- Middleware patterns
- Logging and tracing configuration
- Common error handling

**arche** brings these pieces together into a cohesive, Rust-native foundation,
built on top of well-established libraries and SDKs.

## What arche provides

### `aws`

AWS SDK integrations built on official SDKs:

- **S3**: Client initialization with support for IAM roles or environment-based credentials

### `gcp`

Google Cloud Platform integrations:

- **Drive**: Google Drive client with service account authentication
- **Sheets**: Google Sheets client with service account authentication

### `database`

Database connection management:

- **Postgres**: Connection pooling with `sqlx`, configurable credentials, health checks
- **Redis**: Connection pooling with `bb8`, async operations, health checks

### `jwt`

JWT utilities for authentication and authorization:

- Token generation and verification (HS256)
- Access/refresh token pair generation
- Token expiry helpers
- Custom claims support

### `error`

Axum-compatible error handling:

- `AppError` enum with common HTTP error variants
- Automatic `IntoResponse` conversion
- Structured error responses with details

### `utils`

Common utilities for backend services:

- Timestamp validation and conversion helpers
- `OffsetDateTime` utilities (Unix, ISO8601)
- Pagination parameter types

All components are modular and explicit—nothing is hidden or magical.

## Module Reference

### AWS (`arche::aws`)

#### S3

Initialize an S3 client with automatic credential management:

```rust
use arche::aws::s3::get_s3_client;

let client = get_s3_client().await;
```

**Environment Variables:**

- `S3_CRED_SOURCE`: `"IAM"` (default) or `"env"` for environment-based credentials
- `S3_ACCESS_KEY_ID`: Required when using `"env"` credential source
- `S3_SECRET_ACCESS_KEY`: Required when using `"env"` credential source

### GCP (`arche::gcp`)

#### Drive

```rust
use arche::gcp::drive::get_drive_client;

let drive = get_drive_client().await?;
```

**Environment Variables:**

- `GCP_DRIVE_KEY`: Path to service account JSON key file

#### Sheets

```rust
use arche::gcp::sheets::get_sheets_client;

let sheets = get_sheets_client().await?;
```

**Environment Variables:**

- `GCP_SHEETS_KEY`: Path to service account JSON key file

### Database (`arche::database`)

#### Postgres

```rust
use arche::database::pg::{get_pg_pool, test_pg};

let pool = get_pg_pool().await;
let is_healthy = test_pg(pool.clone()).await;
```

**Environment Variables:**

- `PG_HOST`: Database host
- `PG_PORT`: Database port
- `PG_DATABASE`: Database name
- `PG_MAX_CONN`: Maximum connections in pool
- `PG_CREDENTIALS`: JSON string with `username` and `password` (alternative)
- `PG_USERNAME`: Username (if not using `PG_CREDENTIALS`)
- `PG_PASSWORD`: Password (if not using `PG_CREDENTIALS`)

#### Redis

```rust
use arche::database::redis::{get_redis_pool, test_redis};

let pool = get_redis_pool().await;
let is_healthy = test_redis(pool.clone()).await;
```

**Environment Variables:**

- `REDIS_HOST`: Redis host
- `REDIS_PORT`: Redis port
- `REDIS_MAX_CONN`: Maximum connections in pool

### JWT (`arche::jwt`)

```rust
use arche::jwt::{generate_tokens, verify_token, generate_expiry_time};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

// Generate tokens
let access_claims = Claims { sub: "user_id".into(), exp: generate_expiry_time(3600) };
let refresh_claims = Claims { sub: "user_id".into(), exp: generate_expiry_time(86400) };

let tokens = generate_tokens(
    access_claims,
    refresh_claims,
    &access_secret,
    &refresh_secret,
);

// Verify token
let token_data = verify_token::<Claims>(&token, secret, Some("audience".into()))?;
```

### Error (`arche::error`)

```rust
use arche::error::AppError;
use axum::response::IntoResponse;

async fn handler() -> Result<impl IntoResponse, AppError> {
    Err(AppError::Unauthorized)
}

// Custom errors with details
let error = AppError::bad_request(
    Some(errors_map),
    Some("Invalid input".into()),
    Some("Field validation failed".into()),
);
```

**Error Variants:**

- `Unauthorized` → 401
- `BadRequest` → 400
- `UnprocessableEntity` → 422
- `DBError` → 500
- `InternalError` → 500
- `Unavailable` → 503

### Utils (`arche::utils`)

```rust
use arche::utils::{validate_timestamp, FromOffsetDateTime, PaginationParams};
use sqlx::types::time::OffsetDateTime;

// Timestamp validation
let is_future = validate_timestamp(timestamp, false);

// DateTime conversion
let iso_string = offset_dt.to_iso_string()?;

// Pagination
let params = PaginationParams {
    page_number: Some(1),
    page_size: Some(20),
};
```

## What arche is *not*

- ❌ A framework that replaces Axum
- ❌ A code generator or project template
- ❌ A monolithic abstraction over third-party libraries
- ❌ A "do-everything" utils crate

`arche` favors composition over abstraction.

## Design principles

- **Explicit over implicit**
- **Composition over inheritance**
- **Thin wrappers over official SDKs**
- **Production-first defaults**
- **No global state**
- **Async-first**

## Why arche?

Most backend services end up re-implementing the same infrastructure concerns:

- Cloud SDK setup and ergonomics
- Database connection management
- Authentication primitives
- Middleware patterns
- Logging and tracing configuration
- Common error handling

**arche** brings these pieces together into a cohesive, Rust-native foundation,
built on top of well-established libraries and SDKs.

---

## What arche provides

### `aws`
AWS SDK integrations built on official SDKs:
- **S3**: Client initialization with support for IAM roles or environment-based credentials

### `gcp`
Google Cloud Platform integrations:
- **Drive**: Google Drive client with service account authentication
- **Sheets**: Google Sheets client with service account authentication

### `database`
Database connection management:
- **Postgres**: Connection pooling with `sqlx`, configurable credentials, health checks
- **Redis**: Connection pooling with `bb8`, async operations, health checks

### `jwt`
JWT utilities for authentication and authorization:
- Token generation and verification (HS256)
- Access/refresh token pair generation
- Token expiry helpers
- Custom claims support

### `error`
Axum-compatible error handling:
- `AppError` enum with common HTTP error variants
- Automatic `IntoResponse` conversion
- Structured error responses with details

### `utils`
Common utilities for backend services:
- Timestamp validation and conversion helpers
- `OffsetDateTime` utilities (Unix, ISO8601)
- Pagination parameter types

All components are modular and explicit—nothing is hidden or magical.

---

## Module Reference

### AWS (`arche::aws`)

#### S3
Initialize an S3 client with automatic credential management:
```rust
use arche::aws::s3::get_s3_client;

let client = get_s3_client().await;
```

**Environment Variables:**
- `S3_CRED_SOURCE`: `"IAM"` (default) or `"env"` for environment-based credentials
- `S3_ACCESS_KEY_ID`: Required when using `"env"` credential source
- `S3_SECRET_ACCESS_KEY`: Required when using `"env"` credential source

---

### GCP (`arche::gcp`)

#### Drive
```rust
use arche::gcp::drive::get_drive_client;

let drive = get_drive_client().await?;
```

**Environment Variables:**
- `GCP_DRIVE_KEY`: Path to service account JSON key file

#### Sheets
```rust
use arche::gcp::sheets::get_sheets_client;

let sheets = get_sheets_client().await?;
```

**Environment Variables:**
- `GCP_SHEETS_KEY`: Path to service account JSON key file

---

### Database (`arche::database`)

#### Postgres
```rust
use arche::database::pg::{get_pg_pool, test_pg};

let pool = get_pg_pool().await;
let is_healthy = test_pg(pool.clone()).await;
```

**Environment Variables:**
- `PG_HOST`: Database host
- `PG_PORT`: Database port
- `PG_DATABASE`: Database name
- `PG_MAX_CONN`: Maximum connections in pool
- `PG_CREDENTIALS`: JSON string with `username` and `password` (alternative)
- `PG_USERNAME`: Username (if not using `PG_CREDENTIALS`)
- `PG_PASSWORD`: Password (if not using `PG_CREDENTIALS`)

#### Redis
```rust
use arche::database::redis::{get_redis_pool, test_redis};

let pool = get_redis_pool().await;
let is_healthy = test_redis(pool.clone()).await;
```

**Environment Variables:**
- `REDIS_HOST`: Redis host
- `REDIS_PORT`: Redis port
- `REDIS_MAX_CONN`: Maximum connections in pool

---

### JWT (`arche::jwt`)

```rust
use arche::jwt::{generate_tokens, verify_token, generate_expiry_time};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

// Generate tokens
let access_claims = Claims { sub: "user_id".into(), exp: generate_expiry_time(3600) };
let refresh_claims = Claims { sub: "user_id".into(), exp: generate_expiry_time(86400) };

let tokens = generate_tokens(
    access_claims,
    refresh_claims,
    &access_secret,
    &refresh_secret,
);

// Verify token
let token_data = verify_token::<Claims>(&token, secret, Some("audience".into()))?;
```

---

### Error (`arche::error`)

```rust
use arche::error::AppError;
use axum::response::IntoResponse;

async fn handler() -> Result<impl IntoResponse, AppError> {
    Err(AppError::Unauthorized)
}

// Custom errors with details
let error = AppError::bad_request(
    Some(errors_map),
    Some("Invalid input".into()),
    Some("Field validation failed".into()),
);
```

**Error Variants:**
- `Unauthorized` → 401
- `BadRequest` → 400
- `UnprocessableEntity` → 422
- `DBError` → 500
- `InternalError` → 500
- `Unavailable` → 503

---

### Utils (`arche::utils`)

```rust
use arche::utils::{validate_timestamp, FromOffsetDateTime, PaginationParams};
use sqlx::types::time::OffsetDateTime;

// Timestamp validation
let is_future = validate_timestamp(timestamp, false);

// DateTime conversion
let iso_string = offset_dt.to_iso_string()?;

// Pagination
let params = PaginationParams {
    page_number: Some(1),
    page_size: Some(20),
};
```

---

## What arche is *not*

- ❌ A framework that replaces Axum
- ❌ A code generator or project template
- ❌ A monolithic abstraction over third-party libraries
- ❌ A “do-everything” utils crate

`arche` favors composition over abstraction.

---

## Design principles

- **Explicit over implicit**
- **Composition over inheritance**
- **Thin wrappers over official SDKs**
- **Production-first defaults**
- **No global state**
- **Async-first**

---

