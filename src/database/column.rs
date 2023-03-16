use super::{CellValue, DB};
use anyhow::Result;

pub async fn add_column(
    role_key: [u8; 32],
    table_name: &str,
    column_name: &str,
    encrypted: bool,
) -> Result<()> {
    let default_obj = CellValue::Clear(Vec::new());
    let default_blob = postcard::to_allocvec(&default_obj)?;

    sqlx::query(&format!(
        "
        alter table {table_name}
        add column {column_name} blob default X'{}' 
        ",
        hex::encode(default_blob)
    ))
    .execute(&*DB)
    .await?;
    sqlx::query!(
        "
        insert into smask_role_column 
        (smask_key, smask_table, smask_column)
        select smask_key, smask_table, ?2  as smask_column
        from smask_role_table
        where smask_table =?1

        ",
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;
    let role_key = role_key.to_vec();
    sqlx::query!(
        "
        insert into smask_role_column
        (smask_key, smask_table, smask_column)
        values (?1, ?2, ?3) ",
        role_key,
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;

    if encrypted {
        sqlx::query!(
            "
            insert into smask_encrypted
            (smask_table, smask_column)
            values (?1, ?2)
            ",
            table_name,
            column_name
        )
        .execute(&*DB)
        .await?;
    }
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
        where smask_table = ?1 and smask_column = ?2
        ",
        table_name,
        column_name
    )
    .execute(&*DB)
    .await?;

    sqlx::query!(
        "
        delete from smask_encrypted
        where smask_table = ?1 and smask_column = ?2
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
pub async fn column_encrypted(table_name: &str, column_name: &str) -> Result<bool> {
    Ok(sqlx::query!(
        "
        select count(*) n 
        from smask_encrypted
        where smask_table = ?1
        and smask_column = ?2
        ",
        table_name,
        column_name
    )
    .fetch_one(&*DB)
    .await?
    .n > 0)
}
