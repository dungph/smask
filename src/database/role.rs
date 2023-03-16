use anyhow::Result;
use base58::{FromBase58, ToBase58};
use serde::{Deserialize, Serialize};

use super::{list_column, DB};

pub async fn insert_role(name: &str, pubkey: [u8; 32]) -> Result<()> {
    let pubkey = pubkey.to_vec();
    sqlx::query!(
        "
        insert into smask_role (smask_key, smask_role)
        values (?1, ?2)
            ",
        pubkey,
        name
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

pub async fn role_existed(pubkey: [u8; 32]) -> Result<bool> {
    let pubkey = pubkey.to_vec();
    Ok(sqlx::query!(
        "
        select count(*) as n
        from smask_role
        where smask_key = ?1",
        pubkey
    )
    .fetch_one(&*DB)
    .await?
    .n > 0)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    role_key: String,
    role_name: String,
}
pub async fn list_role() -> Result<Vec<Role>> {
    Ok(sqlx::query!(
        "
        select smask_key, smask_role 
        from smask_role;
        "
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|r| Role {
        role_key: r.smask_key.unwrap_or(Vec::new()).to_base58(),
        role_name: r.smask_role,
    })
    .collect())
}

pub async fn grant_role_table(role_key: &str, table_name: &str) -> Result<()> {
    let role_key: Vec<u8> = role_key.from_base58().unwrap_or(Vec::new());
    sqlx::query!(
        "
        insert into smask_role_table
        (smask_key, smask_table)
        values (?1, ?2)
        ",
        role_key,
        table_name
    )
    .execute(&*DB)
    .await?;
    let cols = list_column(table_name).await?;
    for col in cols {
        sqlx::query!(
            "
            insert into smask_role_column
            (smask_key, smask_table, smask_column)
            values (?1, ?2, ?3)
            ",
            role_key,
            table_name,
            col
        )
        .execute(&*DB)
        .await?;
    }
    Ok(())
}

pub async fn grant_role_column(role_key: &str, table_name: &str, column_name: &str) -> Result<()> {
    let role_key: Vec<u8> = role_key.from_base58().unwrap_or(Vec::new());
    sqlx::query!(
        "
        insert into smask_role_column
        (smask_key, smask_table, smask_column)
        values (?1, ?2, ?3)
        ",
        role_key,
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

pub async fn revoke_role_table(role_key: &str, table_name: &str) -> Result<()> {
    let role_key: Vec<u8> = role_key.from_base58().unwrap_or(Vec::new());
    sqlx::query!(
        "
        delete from smask_role_table
        where smask_key = ?1
        and smask_table = ?2
        ",
        role_key,
        table_name
    )
    .execute(&*DB)
    .await?;
    sqlx::query!(
        "
        delete from smask_role_column
        where smask_key = ?1
        and smask_table = ?2
        ",
        role_key,
        table_name
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

pub async fn revoke_role_column(role_key: &str, table_name: &str, column_name: &str) -> Result<()> {
    let role_key: Vec<u8> = role_key.from_base58().unwrap_or(Vec::new());
    sqlx::query!(
        "
        delete from smask_role_column
        where smask_key = ?1 and smask_table = ?2 and smask_column = ?3
        ",
        role_key,
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;
    Ok(())
}

pub async fn list_role_table(table_name: &str) -> Result<Vec<[u8; 32]>> {
    Ok(sqlx::query!(
        "
        select smask_key
        from smask_role_table
        where smask_table = ?1
        ",
        table_name,
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|r| r.smask_key.try_into().unwrap())
    .collect())
}
pub async fn list_role_column(table_name: &str, column_name: &str) -> Result<Vec<[u8; 32]>> {
    Ok(sqlx::query!(
        "
        select smask_key
        from smask_role_column
        where smask_table = ?1 and smask_column = ?2
        ",
        table_name,
        column_name
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|r| r.smask_key.try_into().unwrap())
    .collect())
}
