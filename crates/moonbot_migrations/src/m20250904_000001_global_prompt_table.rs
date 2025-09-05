use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GlobalPrompt::Table)
                    .if_not_exists()
                    .col(big_unsigned_uniq(GlobalPrompt::Id).primary_key())
                    .col(text(GlobalPrompt::PromptJson))
                    .col(timestamp(GlobalPrompt::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GlobalPrompt::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GlobalPrompt {
    Table,
    Id,
    PromptJson,
    UpdatedAt,
}
