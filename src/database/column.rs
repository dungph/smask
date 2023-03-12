use anyhow::Result;
use sqlx::Row;

use super::{cell, DB};

pub async fn add_column(role_key: [u8; 32], table_name: &str, column_name: &str) -> Result<()> {
    sqlx::query(&format!(
        "
        alter table {table_name}
        add column {column_name} integer
        "
    ))
    .execute(&*DB)
    .await?;
    let rowids = sqlx::query(&format!(
        "
        select rowid from {table_name}
        "
    ))
    .fetch_all(&*DB)
    .await?
    .iter()
    .map(|r| r.get("rowid"))
    .collect::<Vec<i64>>();
    for rowid in rowids {
        let cell_id = cell::insert_cell(table_name, column_name).await?;
        sqlx::query(&format!(
            "
            update {table_name}
            set {column_name} = {cell_id}
            where rowid = {rowid}
            "
        ))
        .execute(&*DB)
        .await?;
    }

    let role_key = role_key.to_vec();
    sqlx::query!(
        "
        insert into smask_role_column
        (smask_role_key, smask_table_name, smask_column_name)
        values (?1, ?2, ?3) ",
        role_key,
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;
    return Ok(());
}

pub async fn drop_column(table_name: &str, column_name: &str) -> Result<()> {
    sqlx::query(&format!(
        "
        alter table {table_name}
        drop column {column_name}
        "
    ))
    .execute(&*DB)
    .await?;
    sqlx::query!(
        "
        delete from smask_role_column
        where smask_table_name = ?1 and smask_column_name = ?2
        ",
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;
    return Ok(());
}

pub async fn list_column(table_name: &str) -> Result<Vec<String>> {
    let columns = sqlx::query!(
        "
        select sql
        from sqlite_master
        where name = ?1",
        table_name
    )
    .fetch_one(&*DB)
    .await?
    .sql
    .map(|s| s.split_once('(').map(|(_, s)| s.to_owned()))
    .flatten()
    .map(|s| {
        s.split(',')
            .map(|c| c.trim().split_once(' ').unwrap())
            .map(|(s, _)| s.to_string())
            .filter(|s| s != "rowid")
            .collect::<Vec<String>>()
    })
    .unwrap_or_else(|| Vec::new());
    Ok(columns)
}
