use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};

use crate::CellValue;

use super::{cell, column, DB};

#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    pub rowid: i64,
    pub cols: BTreeMap<String, CellValue>,
}

pub async fn table_data(table_name: &str) -> Result<Vec<Record>> {
    let columns = column::list_column(table_name).await?;
    Ok(sqlx::query(format!("select * from {table_name}").as_str())
        .fetch_all(&*DB)
        .await?
        .iter()
        .map(|row| {
            let mut ret = BTreeMap::new();
            for col in columns.iter().filter(|c| c.as_str() != "rowid") {
                let raw: Vec<u8> = row.get(col.as_str());
                let val = postcard::from_bytes(&raw).unwrap();
                ret.insert(col.to_owned(), val);
            }
            let rowid: i64 = row.get("rowid");
            Record { rowid, cols: ret }
        })
        .collect())
}

pub async fn create_table(role_key: [u8; 32], table_name: &str) -> Result<()> {
    sqlx::query(&format!(
        "
        create table {table_name}
        (rowid integer primary key)"
    ))
    .execute(&*DB)
    .await?;

    let role_key = role_key.to_vec();
    sqlx::query!(
        "
        insert into smask_role_table
        (smask_key, smask_table)
        values(?1, ?2)
        ",
        role_key,
        table_name
    )
    .execute(&*DB)
    .await?;
    return Ok(());
}

pub async fn drop_table(table_name: &str) -> Result<()> {
    sqlx::query(&format!("drop table {table_name}"))
        .execute(&*DB)
        .await?;

    sqlx::query!(
        "
        delete from smask_role_table
        where smask_table = ?1
        ",
        table_name
    )
    .execute(&*DB)
    .await?;

    sqlx::query!(
        "
        delete from smask_role_column
        where smask_table = ?1
        ",
        table_name
    )
    .execute(&*DB)
    .await?;
    return Ok(());
}

pub async fn clear_table(table_name: &str) -> Result<()> {
    let query = format!("delete from {table_name}");
    sqlx::query(&query).execute(&*DB).await?;
    return Ok(());
}

pub async fn list_table() -> Result<Vec<String>> {
    const BUILT_IN_TABLE: &[&str] = &[
        "_sqlx_migrations",
        "sqlite_autoindex__sqlx_migrations_1",
        "smask_role",
        "sqlite_autoindex_smask_role_1",
        "smask_role_table",
        "smask_role_column",
        "smask_role_column_value",
        "smask_encrypted",
    ];
    let tables = sqlx::query!(
        "
        select name
        from sqlite_master
        ",
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .filter_map(|r| r.name)
    .filter(|s| !BUILT_IN_TABLE.iter().any(|t| t == s))
    .collect::<Vec<String>>();
    Ok(tables)
}

pub async fn new_record(table_name: &str) -> Result<i64> {
    Ok(sqlx::query(&format!(
        "
        insert into {table_name}
        default values
        returning rowid"
    ))
    .fetch_one(&*DB)
    .await?
    .get("rowid"))
}
pub async fn remove_record(table_name: &str, record: i64) -> Result<()> {
    sqlx::query(&format!(
        "
        delete from {table_name}
        where rowid = ?1"
    ))
    .bind(record)
    .execute(&*DB)
    .await?;
    Ok(())
}
