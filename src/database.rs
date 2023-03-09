use once_cell::sync::Lazy;

static DB: Lazy<sqlx::sqlite::SqlitePool> = Lazy::new(|| {
    sqlx::sqlite::SqlitePool::connect_lazy(&std::env::var("DATABASE_URL").unwrap()).unwrap()
});

async fn migrate() -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(&*DB).await
}

async fn create_table(
    table_name: &str,
    column_name: impl IntoIterator<Item = &str>,
) -> anyhow::Result<()> {
    let query = format!("create table {table_name} (rowid integer primary key)");
    sqlx::query(&query).execute(&*DB).await?;
    return Ok(());
}
async fn add_column(table_name: &str, column_name: &str) -> anyhow::Result<()> {
    let query = format!("alter table {table_name} add column {column_name} integer not null");
    sqlx::query(&query).execute(&*DB).await?;
    return Ok(());
}
async fn drop_column(table_name: &str, column_name: &str) -> anyhow::Result<()> {
    let query = format!("alter table {table_name} drop column {column_name}");
    sqlx::query(&query).execute(&*DB).await?;
    return Ok(());
}
