use std::collections::BTreeMap;

mod database;
mod session;

use serde::Deserialize;
use tera::Tera;
use tide::{Middleware, Next, Request, Response};
use tide_tera::prelude::*;

use crate::database::CellValue;

const ROLE_PUBKEY: &str = "ROLE_PUBKEY";
const ROLE_KEY: &str = "ROLE_KEY";

fn te() -> Tera {
    let mut tera = Tera::new("templates/**/*").unwrap();
    tera.autoescape_on(vec!["html"]);
    tera
}

struct M;

#[tide::utils::async_trait]
impl Middleware<()> for M {
    async fn handle(&self, req: Request<()>, next: Next<'_, ()>) -> tide::Result {
        match req.url().path() {
            "/" | "/login" | "/signup" => {
                return Ok(next.run(req).await);
            }
            _ => {}
        }
        if let Some(key) = req.session().get(ROLE_KEY) {
            let key = x25519_dalek::x25519(key, x25519_dalek::X25519_BASEPOINT_BYTES);
            if database::role_existed(key).await? {
                return Ok(next.run(req).await);
            }
        }
        Ok(tide::Redirect::new("/").into())
    }
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    tide::log::start();
    database::migrate().await?;

    let mut server = tide::new();

    server.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        &rand::random::<[u8; 32]>(),
    ));

    server.with(M);

    server.at("/").get(|req: Request<()>| async move {
        te().render_response("hello.html", &tera::Context::new())
    });
    server.at("/signup").post(session::signup);
    server.at("/login").post(session::login);
    server.at("/logout").post(session::logout);

    server.at("/table/all").get(|req: Request<()>| async move {
        let mut cx = tera::Context::new();
        let tables = database::list_table().await?;
        let roles = database::list_role().await?;
        cx.insert("table_names", &tables);
        cx.insert("roles", &roles);
        te().render_response("table-all.html", &cx)
    });

    server
        .at("/table/new")
        .post(|mut req: Request<()>| async move {
            #[derive(Deserialize)]
            struct Query {
                table_name: String,
            }
            let table_name: Query = req.body_form().await?;

            database::create_table(
                req.session().get(ROLE_PUBKEY).unwrap(),
                &table_name.table_name,
            )
            .await?;

            Ok(tide::Redirect::new("/table/all"))
        });

    server
        .at("/table/:table_name/drop")
        .post(|req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;

            database::drop_table(&table_name).await?;
            Ok(tide::Redirect::new("/table/all"))
        });

    server
        .at("/table/:table_name/clear")
        .post(|req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;

            database::clear_table(&table_name).await?;
            Ok(tide::Redirect::new("/table/all"))
        });

    server
        .at("/table/:table_name/view")
        .get(|req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;

            let mut cx = tera::Context::new();
            let column_names = database::list_column(&table_name).await?;
            let table_datas = database::table_data(&table_name).await?;
            let reveal_list: BTreeMap<String, u8> = req.query()?;

            let table_datas = table_datas
                .into_iter()
                .map(|rec| {
                    let rowid = rec.rowid;
                    let rowid_str = rec.rowid.to_string();
                    rec.cols
                        .into_iter()
                        .map(|(k, v)| {
                            let rev_key = format!("view-{k}-{rowid}");
                            if let Some(1) = reveal_list.get(&rev_key) {
                                let raw = v.decrypt(req.session().get(ROLE_KEY).unwrap());
                                if let Some(val) = raw {
                                    (k, String::from_utf8_lossy(&val).to_string())
                                } else {
                                    (k, String::from("[Decryption failed]"))
                                }
                            } else {
                                let s = match v {
                                    CellValue::Clear(v) => String::from_utf8_lossy(&v).to_string(),
                                    _ => String::from("[Encrypted]"),
                                };
                                (k, s)
                            }
                        })
                        .chain([(String::from("rowid"), rowid_str)])
                        .collect()
                })
                .collect::<Vec<BTreeMap<String, String>>>();

            let roles = database::list_role().await?;
            cx.insert("roles", &roles);
            cx.insert("column_names", &column_names);
            cx.insert("table_datas", &table_datas);
            cx.insert("table_name", &table_name);
            te().render_response("table-view.html", &cx)
        });

    server
        .at("/table/:table_name/column/:column_name/record/:record/update")
        .post(|mut req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            let column_name = req.param("column_name")?.parse::<String>()?;
            let record = req.param("record")?.parse::<i64>()?;
            #[derive(Deserialize)]
            struct Query {
                value: String,
            }
            let query: Query = req.body_form().await?;

            let cell_value = if database::column_encrypted(&table_name, &column_name).await? {
                let readers = database::list_role_column(&table_name, &column_name).await?;
                CellValue::encrypted(query.value.as_bytes(), &readers)
            } else {
                CellValue::plain(query.value.as_bytes())
            };
            database::update_cell(&table_name, &column_name, record, &cell_value).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });
    server
        .at("/table/:table_name/record/new")
        .post(|req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            let _ = database::new_record(&table_name).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });

    server
        .at("/table/:table_name/record/:record/drop")
        .post(|req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            let record = req.param("record")?.parse::<i64>()?;
            database::remove_record(&table_name, record).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });

    server
        .at("/table/:table_name/column/new")
        .post(|mut req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            #[derive(Deserialize)]
            struct Query {
                column_name: String,
                encrypted: Option<String>,
            }
            let query: Query = req.body_form().await?;

            database::add_column(
                req.session().get(ROLE_PUBKEY).unwrap(),
                &table_name,
                &query.column_name,
                query.encrypted.is_some(),
            )
            .await?;

            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });

    server
        .at("/table/:table_name/column/:column_name/drop")
        .post(|req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            let column_name = req.param("column_name")?.parse::<String>()?;
            database::drop_column(&table_name, &column_name).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });
    server
        .at("/role/table/:table_name/new")
        .post(|mut req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            #[derive(Deserialize)]
            struct Query {
                role_key: String,
            }

            let query: Query = req.body_form().await?;

            database::grant_role_table(query.role_key.as_str(), &table_name).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });
    server
        .at("/role/table/:table_name/drop")
        .post(|mut req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            #[derive(Deserialize)]
            struct Query {
                role_key: String,
            }

            let query: Query = req.body_form().await?;

            database::revoke_role_table(query.role_key.as_str(), &table_name).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });
    server
        .at("/role/table/:table_name/column/:column_name/new")
        .post(|mut req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            let column_name = req.param("column_name")?.parse::<String>()?;
            #[derive(Deserialize)]
            struct Query {
                role_key: String,
            }

            let query: Query = req.body_form().await?;

            database::grant_role_column(query.role_key.as_str(), &table_name, &column_name).await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });
    server
        .at("/role/table/:table_name/column/:column_name/drop")
        .post(|mut req: Request<()>| async move {
            let table_name = req.param("table_name")?.parse::<String>()?;
            let column_name = req.param("column_name")?.parse::<String>()?;
            #[derive(Deserialize)]
            struct Query {
                role_key: String,
            }

            let query: Query = req.body_form().await?;

            database::revoke_role_column(query.role_key.as_str(), &table_name, &column_name)
                .await?;
            Ok(tide::Redirect::new(format!("/table/{table_name}/view")))
        });

    server.listen("0.0.0.0:8080").await?;
    Ok(())
}
