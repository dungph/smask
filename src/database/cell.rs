use std::collections::BTreeMap;

use anyhow::Result;
use base58::ToBase58;
use blake2::Digest;
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, ChaChaPoly1305, KeyInit};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use x25519_dalek::{x25519, X25519_BASEPOINT_BYTES};

use super::DB;

#[derive(Serialize, Deserialize, Debug)]
pub enum CellValue {
    Clear(Vec<u8>),
    Encrypted {
        ephemeral: [u8; 32],
        value: BTreeMap<[u8; 32], Vec<u8>>,
    },
    KeyEncrypted {
        ephemeral: [u8; 32],
        keys: BTreeMap<[u8; 32], Vec<u8>>,
        cipher: Vec<u8>,
    },
}

impl CellValue {
    pub fn plain(data: &[u8]) -> Self {
        Self::Clear(data.to_owned())
    }
    pub fn encrypted(data: &[u8], readers: &[[u8; 32]]) -> Self {
        let e = rand::random::<[u8; 32]>();
        let pub_e = x25519(e, X25519_BASEPOINT_BYTES);
        let mut values = BTreeMap::new();
        for reader in readers {
            let shared = x25519(e, *reader);
            let mut hasher = blake2::Blake2s256::new();
            hasher.update(shared);
            let cipher = ChaCha20Poly1305::new(&hasher.finalize());
            let ciphertext = cipher.encrypt(&[0u8; 12].into(), data).unwrap();
            values.insert(*reader, ciphertext);
        }
        Self::Encrypted {
            ephemeral: pub_e,
            value: values,
        }
    }
    pub fn cipher_encrypted(data: &[u8], key: [u8; 32], readers: &[[u8; 32]]) -> Self {
        match Self::encrypted(key.as_slice(), readers) {
            CellValue::Encrypted { ephemeral, value } => {
                let cipher = ChaCha20Poly1305::new(&key.into());
                let blob = cipher.encrypt(&[0u8; 12].into(), data).unwrap();
                CellValue::KeyEncrypted {
                    ephemeral,
                    keys: value,
                    cipher: blob,
                }
            }
            _ => unreachable!(),
        }
    }
    pub fn decrypt(self, reader: [u8; 32]) -> Option<Vec<u8>> {
        match self {
            CellValue::Clear(v) => Some(v),
            CellValue::Encrypted { ephemeral, value } => {
                let pubkey = x25519(reader, X25519_BASEPOINT_BYTES);
                let val = value.get(&pubkey)?;
                let shared = x25519(reader, ephemeral);
                let mut hasher = blake2::Blake2s256::new();
                hasher.update(shared);
                let cipher = ChaCha20Poly1305::new(&hasher.finalize());
                let ret = cipher.decrypt(&[0u8; 12].into(), val.as_slice()).ok()?;
                Some(ret)
            }
            CellValue::KeyEncrypted {
                ephemeral,
                keys,
                cipher: blob,
            } => {
                let pubkey = x25519(reader, X25519_BASEPOINT_BYTES);
                let encrypted_key = keys.get(&pubkey)?;
                let shared = x25519(reader, ephemeral);
                let mut hasher = blake2::Blake2s256::new();
                hasher.update(shared);
                let cipher = ChaCha20Poly1305::new(&hasher.finalize());
                let key: [u8; 32] = cipher
                    .decrypt(&[0u8; 12].into(), encrypted_key.as_slice())
                    .ok()?
                    .try_into()
                    .ok()?;

                let blob_cipher = ChaCha20Poly1305::new(&key.into());
                blob_cipher.decrypt(&[0u8; 12].into(), blob.as_slice()).ok()
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
    let raw = postcard::to_allocvec(data)?;
    sqlx::query(&format!(
        "
        update {table_name}
        set {column_name} = ?2 
        where rowid = ?1
            ",
    ))
    .bind(rowid)
    .bind(raw)
    .execute(&*DB)
    .await?;

    Ok(())
}
