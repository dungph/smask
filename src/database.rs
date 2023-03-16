use std::collections::BTreeMap;

use anyhow::Result;
use base58::ToBase58;
use blake2::Digest;
use chacha20poly1305::{aead::Aead, AeadInPlace, KeyInit};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use x25519_dalek::{x25519, X25519_BASEPOINT_BYTES};

static DB: Lazy<sqlx::sqlite::SqlitePool> = Lazy::new(|| {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_lazy(
            &std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db".to_string()),
        )
        .unwrap()
});

pub async fn migrate() -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(&*DB).await
}
pub mod cell;
pub mod column;
pub mod role;
pub mod table;

pub use cell::*;
pub use column::*;
pub use role::*;
pub use table::*;
