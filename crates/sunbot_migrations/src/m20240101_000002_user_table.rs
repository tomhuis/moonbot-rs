use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(big_unsigned_uniq(User::UserId).primary_key())
                    .col(string(User::Username))
                    .col(string_null(User::DisplayName))
                    .col(float(User::Temperature).default(0.0))
                    .col(json(User::Keywords).default(Expr::value("[]")))
                    .col(integer(User::InteractionCount).default(0))
                    .col(timestamp_null(User::LastInteraction))
                    .col(text_null(User::RelationshipNotes))
                    .col(timestamp(User::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(User::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
    Username,
    DisplayName,
    Temperature,
    Keywords,
    InteractionCount,
    LastInteraction,
    RelationshipNotes,
    CreatedAt,
    UpdatedAt,
}