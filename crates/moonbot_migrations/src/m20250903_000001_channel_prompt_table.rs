use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChannelPrompt::Table)
                    .if_not_exists()
                    .col(big_unsigned_uniq(ChannelPrompt::ChannelId).primary_key())
                    .col(text(ChannelPrompt::PromptJson))
                    .col(timestamp(ChannelPrompt::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChannelPrompt::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ChannelPrompt {
    Table,
    ChannelId,
    PromptJson,
    UpdatedAt,
}
