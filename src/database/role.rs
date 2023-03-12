use anyhow::Result;
use base58::ToBase58;
use serde::{Deserialize, Serialize};

use super::DB;

pub async fn insert_role(name: &str, pubkey: [u8; 32]) -> Result<()> {
    let pubkey = pubkey.to_vec();
    sqlx::query!(
        "
        insert into smask_role (smask_role_key, smask_role_name)
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
        where smask_role_key = ?1",
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
        select smask_role_key, smask_role_name 
        from smask_role;
        "
    )
    .fetch_all(&*DB)
    .await?
    .into_iter()
    .map(|r| Role {
        role_key: r.smask_role_key.unwrap_or(Vec::new()).to_base58(),
        role_name: r.smask_role_name,
    })
    .collect())
}
