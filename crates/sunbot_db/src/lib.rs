use sea_orm::prelude::*;
use sea_orm::{ConnectOptions, Database};
use sunbot_migrations::{Migrator, MigratorTrait};
use tokio::sync::OnceCell;

static DB_CLIENT: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub mod entities;
pub mod services;

pub async fn init_db(database_url: &str) {
    let opt = ConnectOptions::new(database_url);
    let db = Database::connect(opt).await.unwrap();
    DB_CLIENT
        .set(db)
        .unwrap_or_else(|_| panic!("don't call `init_db()` more than once"));

    Migrator::up(get_db().await, None).await.unwrap();
}

pub async fn get_db() -> &'static DatabaseConnection {
    DB_CLIENT
        .get()
        .expect("called `get_db()` before db was initialized")
}
