pub use sea_orm_migration::prelude::*;

mod m20220101_000001_guild_table;
mod m20250903_000001_channel_prompt_table;
mod m20250904_000001_global_prompt_table;
mod m20250904_000002_bot_disposition_table;
mod m20250904_000003_user_insight_table;
mod m20250905_000001_user_directory_rag;
mod m20250905_000002_channel_roleplay_table;
mod m20250905_000004_guild_roleplay_table;
mod m20250905_000005_corpus_rag;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_guild_table::Migration),
            Box::new(m20250903_000001_channel_prompt_table::Migration),
            Box::new(m20250904_000001_global_prompt_table::Migration),
            Box::new(m20250904_000002_bot_disposition_table::Migration),
            Box::new(m20250904_000003_user_insight_table::Migration),
            Box::new(m20250905_000001_user_directory_rag::Migration),
            Box::new(m20250905_000002_channel_roleplay_table::Migration),
            Box::new(m20250905_000004_guild_roleplay_table::Migration),
            Box::new(m20250905_000005_corpus_rag::Migration),
        ]
    }
}
