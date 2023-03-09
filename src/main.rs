use std::{collections::BTreeMap, sync::Arc};

mod database;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::{query, SqlitePool};
use tera::Tera;
use tide::{Request, Response};
use tide_tera::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum CellValue {
    Clear(Vec<u8>),
    Encrypted {
        ephemeral: [u8; 32],
        value: BTreeMap<[u8; 32], Vec<u8>>,
    },
}

#[derive(Clone)]
struct State {
    sqlite: SqlitePool,
    tera: Tera,
}
fn te() -> Tera {
    Tera::new("templates/**/*").unwrap()
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    tide::log::start();
    dotenvy::dotenv()?;

    let sqlite = sqlx::sqlite::SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&sqlite).await?;

    let mut tera = Tera::new("templates/**/*")?;
    tera.autoescape_on(vec!["html"]);

    let mut server = tide::with_state(State { sqlite, tera });

    server.at("/").get(|req: Request<State>| async move {
        let tera = &req.state().tera;
        let name = "h";
        tera.render_response("hello.html", &context! {"name" => name})
    });
    server
        .at("/table/all")
        .get(|req: Request<State>| async move {
            let state = req.state();
            let mut cx = tera::Context::new();

            #[derive(Serialize)]
            struct Table {
                id: i64,
                name: String,
            }
            let table = query!("select smask_table_id, smask_table_name from smask_table")
                .fetch_all(&state.sqlite)
                .await?
                .into_iter()
                .map(|r| Table {
                    id: r.smask_table_id,
                    name: r.smask_table_name,
                })
                .collect::<Vec<Table>>();
            cx.insert("tables", &table);
            te().render_response("table-all.html", &cx)
        });
    server
        .at("/table/schema/:table_id")
        .get(|req: Request<State>| async move {
            let table_id = req.param("table_id")?.parse::<u64>()?;

            let state = req.state();
            let mut cx = tera::Context::new();

            #[derive(Serialize)]
            struct Column {
                id: i64,
                name: String,
            }
            let columns = query!("select smask_column_id, smask_column_name from smask_column")
                .fetch_all(&state.sqlite)
                .await?
                .into_iter()
                .map(|r| Column {
                    id: r.smask_column_id,
                    name: r.smask_column_name,
                })
                .collect::<Vec<Column>>();
            cx.insert("columns", &columns);
            cx.insert("table_id", &table_id);
            te().render_response("table-schema.html", &cx)
        });
    server.listen("0.0.0.0:8080").await?;
    Ok(())
}
