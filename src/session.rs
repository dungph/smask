use blake2::Digest;
use serde::Deserialize;
use tide::Request;

use crate::{database, ROLE_KEY, ROLE_PUBKEY};

pub async fn signup(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Query {
        role_name: String,
        role_passphrase: String,
        role_passphrase_2: String,
    }

    let query: Query = req.body_form().await?;
    if query.role_passphrase == query.role_passphrase_2 {
        let mut hash = blake2::Blake2s256::new();
        hash.update(&query.role_name);
        hash.update(query.role_passphrase);
        let private_key: [u8; 32] = hash.finalize().into();
        let public_key = x25519_dalek::x25519(private_key, x25519_dalek::X25519_BASEPOINT_BYTES);
        database::insert_role(&query.role_name, public_key).await?;
    }
    Ok(tide::Redirect::new("/").into())
}
pub async fn logout(mut req: Request<()>) -> tide::Result {
    req.session_mut().remove(ROLE_KEY);
    req.session_mut().remove(ROLE_PUBKEY);
    Ok(tide::Redirect::new("/").into())
}
pub async fn login(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Query {
        role_name: String,
        role_passphrase: String,
    }

    let query: Query = req.body_form().await?;
    let mut hash = blake2::Blake2s256::new();
    hash.update(&query.role_name);
    hash.update(query.role_passphrase);
    let private_key: [u8; 32] = hash.finalize().into();
    let public_key = x25519_dalek::x25519(private_key, x25519_dalek::X25519_BASEPOINT_BYTES);
    if database::role_existed(public_key).await? {
        let session = req.session_mut();
        session.insert(ROLE_KEY, private_key)?;
        session.insert(ROLE_PUBKEY, public_key)?;
        Ok(tide::Redirect::new("/table/all").into())
    } else {
        Ok(tide::Redirect::new("/").into())
    }
}
