use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::{cell, column, DB};

#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    rowid: i64,
    pub cols: Vec<i64>,
}

pub async fn table_data(table_name: &str) -> Result<Vec<Record>> {
    let columns = column::list_column(table_name).await?;
    Ok(sqlx::query(format!("select * from {table_name}").as_str())
        .fetch_all(&*DB)
        .await?
        .iter()
        .map(|row| {
            let rowid = row.get("rowid");
            let cols = columns
                .iter()
                .map(|column| row.get(column.as_str()))
                .collect();
            Record { rowid, cols }
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
        (smask_role_key, smask_table_name)
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
    let query = format!("drop table {table_name}");
    sqlx::query(&query).execute(&*DB).await?;
    sqlx::query!(
        "
        delete from smask_role_table
        where smask_table_name = ?1
        ",
        table_name
    )
    .execute(&*DB)
    .await?;

    sqlx::query!(
        "
        delete from smask_role_column
        where smask_table_name = ?1
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

pub async fn list_table(role_key: [u8; 32]) -> Result<Vec<String>> {
    const BUILT_IN_TABLE: &[&str] = &[
        "_sqlx_migrations",
        "sqlite_autoindex__sqlx_migrations_1",
        "smask_role",
        "sqlite_autoindex_smask_role_1",
        "smask_role_table",
        "smask_role_column",
        "smask_role_column_value",
        "smask_cell",
    ];
    let role_key = role_key.to_vec();
    let tables = sqlx::query!(
        "
        select name
        from sqlite_master
        join smask_role_table
        on smask_table_name = name
        where smask_role_key = ?1",
        role_key
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
    let column_list = column::list_column(table_name).await?;
    let columns = column_list.join(",");
    let mut values: Vec<i64> = vec![];
    for c in column_list {
        values.push(cell::insert_cell(table_name, &c).await?);
    }
    let values_str = values
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let rowid = sqlx::query(&format!(
        "
        insert into {table_name}
        ({columns})
        values ({values_str})
        returning rowid"
    ))
    .fetch_one(&*DB)
    .await?
    .get("rowid");
    Ok(rowid)
}
pub async fn remove_record(table_name: &str, record: i64) -> Result<()> {
    sqlx::query(&format!(
        "
        delete from {table_name}
        where rowid = {record}"
    ))
    .execute(&*DB)
    .await?;
    Ok(())
}
