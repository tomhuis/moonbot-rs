pub use sea_orm_migration::prelude::*;

mod m20220101_000001_guild_table;
mod m20250903_000001_channel_prompt_table;
mod m20250904_000001_global_prompt_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_guild_table::Migration),
            Box::new(m20250903_000001_channel_prompt_table::Migration),
            Box::new(m20250904_000001_global_prompt_table::Migration),
        ]
    }
}
