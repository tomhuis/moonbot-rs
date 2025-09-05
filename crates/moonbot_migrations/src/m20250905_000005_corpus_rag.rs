use sea_orm_migration::{prelude::*, schema::*, sea_orm::{Statement, DbBackend}};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Base corpus table
        manager
            .create_table(
                Table::create()
                    .table(Corpus::Table)
                    .if_not_exists()
                    .col(big_integer(Corpus::Id).auto_increment().primary_key())
                    .col(big_integer(Corpus::GuildId).null())
                    .col(big_integer(Corpus::ChannelId).null())
                    .col(big_integer(Corpus::UserId).null())
                    .col(string_len(Corpus::Kind, 64))
                    .col(text(Corpus::Content))
                    .col(timestamp(Corpus::CreatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // FTS5 table with external content
        let db = manager.get_connection();
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE VIRTUAL TABLE IF NOT EXISTS corpus_fts USING fts5(content, content_rowid='rowid')".to_string(),
        )).await?;

        // Triggers
        for (name, body) in [
            (
                "corpus_fts_ai",
                "INSERT INTO corpus_fts(rowid, content) VALUES (new.rowid, coalesce(new.kind,'') || ' ' || coalesce(new.content,''));",
            ),
            (
                "corpus_fts_ad",
                "INSERT INTO corpus_fts(corpus_fts, rowid, content) VALUES('delete', old.rowid, '');",
            ),
            (
                "corpus_fts_au",
                "INSERT INTO corpus_fts(corpus_fts, rowid, content) VALUES('delete', old.rowid, '');
                 INSERT INTO corpus_fts(rowid, content) VALUES (new.rowid, coalesce(new.kind,'') || ' ' || coalesce(new.content,''));",
            ),
        ] {
            let sql = format!(
                "CREATE TRIGGER IF NOT EXISTS {name} AFTER {event} ON corpus BEGIN {body} END;",
                name = name,
                event = if name.ends_with("_ai") { "INSERT" } else if name.ends_with("_ad") { "DELETE" } else { "UPDATE" },
                body = body
            );
            db.execute(Statement::from_string(DbBackend::Sqlite, sql)).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for name in ["corpus_fts_ai", "corpus_fts_ad", "corpus_fts_au"] {
            let sql = format!("DROP TRIGGER IF EXISTS {name};");
            db.execute(Statement::from_string(DbBackend::Sqlite, sql)).await?;
        }
    db.execute(Statement::from_string(DbBackend::Sqlite, "DROP TABLE IF EXISTS corpus_fts".to_string())).await?;
        manager
            .drop_table(Table::drop().table(Corpus::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Corpus { Table, Id, GuildId, ChannelId, UserId, Kind, Content, CreatedAt }

// FTS table created via raw SQL
