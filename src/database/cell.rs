use std::collections::BTreeMap;

use anyhow::Result;
use base58::ToBase58;
use blake2::Digest;
use chacha20poly1305::{aead::Aead, KeyInit};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use x25519_dalek::{x25519, X25519_BASEPOINT_BYTES};

use super::DB;

#[derive(Serialize, Deserialize, Debug)]
pub enum CellValue {
    Clear(Vec<u8>),
    Encrypted {
        ephemeral: [u8; 32],
        value: BTreeMap<String, Vec<u8>>,
    },
}

impl CellValue {
    pub fn encrypt(readers: &[[u8; 32]], data: &[u8]) -> Self {
        let e = rand::random::<[u8; 32]>();
        let pub_e = x25519(e, X25519_BASEPOINT_BYTES);
        let mut values = BTreeMap::new();
        for reader in readers {
            let shared = x25519(e, *reader);
            let mut hasher = blake2::Blake2s256::new();
            hasher.update(shared);
            let cipher = chacha20poly1305::ChaCha20Poly1305::new(&hasher.finalize());
            let ciphertext = cipher.encrypt(&[0u8; 12].into(), data).unwrap();
            values.insert(reader.to_base58(), ciphertext);
        }
        Self::Encrypted {
            ephemeral: pub_e,
            value: values,
        }
    }
    pub fn decrypt(self, reader: [u8; 32]) -> Option<Vec<u8>> {
        match self {
            CellValue::Clear(v) => Some(v),
            CellValue::Encrypted { ephemeral, value } => {
                let pubkey = x25519(reader, X25519_BASEPOINT_BYTES);
                let val = value.get(&pubkey.to_base58())?;
                let shared = x25519(reader, ephemeral);
                let mut hasher = blake2::Blake2s256::new();
                hasher.update(shared);
                let cipher = chacha20poly1305::ChaCha20Poly1305::new(&hasher.finalize());
                Some(cipher.decrypt(&[0u8; 12].into(), val.as_slice()).ok()?)
            }
        }
    }
}

pub async fn update_cell(
    table_name: &str,
    column_name: &str,
    rowid: i64,
    data: &CellValue,
) -> Result<()> {
    let rowid: i64 = sqlx::query(&format!(
        "
        select {column_name}
        from {table_name}
        where rowid = {rowid}"
    ))
    .fetch_one(&*DB)
    .await
    .map(|r| r.get(column_name))?;

    let raw = serde_json::to_vec(data)?;
    sqlx::query!(
        "
        update smask_cell
        set smask_cell_data=?2
        where smask_cell_id=?1",
        rowid,
        raw
    )
    .execute(&*DB)
    .await?;
    Ok(())
}
pub async fn insert_cell(table_name: &str, column_name: &str) -> Result<i64> {
    Ok(sqlx::query!(
        "
        insert into smask_cell
        (smask_cell_table, smask_cell_column, smask_cell_data)
        values (?1, ?2, X'')
        returning smask_cell_id",
        table_name,
        column_name
    )
    .fetch_one(&*DB)
    .await?
    .smask_cell_id)
}
pub async fn get_cell(cell_id: i64) -> Result<CellValue> {
    let raw = sqlx::query!(
        "
        select smask_cell_data
        from smask_cell
        where smask_cell_id = ?1",
        cell_id
    )
    .fetch_one(&*DB)
    .await?
    .smask_cell_data;

    Ok(serde_json::from_slice(&raw)?)
}
