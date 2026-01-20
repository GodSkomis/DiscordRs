use sqlx::{Error, FromRow, PgPool, Row};


#[derive(Debug, FromRow)]
pub struct AutoRoom {
    pub channel_id: i64,
    pub guild_id: i64,
    pub category_id: i64,
    pub suffix: String
}

impl AutoRoom {
    pub fn to_display_string(&self) -> String {
        format!(
            "ChannelID: {}, Category: {}, Suffix: {}",
            self.channel_id,
            self.category_id,
            self.suffix
        )
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AutoRoomDeleteStrategy<'a> {
    SingleByChannelId(i64),
    SingleByCategoryId(i64),
    ManyByChannelId(&'a Vec<i64>),
    ManyByCategoryId(&'a Vec<i64>)
}

#[derive(Debug, FromRow)]
pub struct MonitoredAutoRoom {
    pub channel_id: i64,
    pub owner_id: i64
}

impl AutoRoom {
    pub async fn get_by_channel_id(pool: &PgPool, channel_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, AutoRoom>("SELECT channel_id, guild_id, category_id, suffix from autoroom WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_one(pool)
            .await {
            Ok(autoroom) => Ok(Some(autoroom)),
            Err(err) => match err {
                sqlx::Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }

    pub async fn create(&self, pool: &PgPool) -> Result<(), &'static str> {
        let query = "INSERT INTO autoroom (channel_id, guild_id, category_id, suffix) VALUES ($1, $2, $3, $4)";
        tracing::info!(
            "Inserting AutoRoom, CHANNEL({}) GUILD({}) CATEGORY({}) SUFFIX({})",
            self.channel_id,
            self.guild_id,
            self.category_id,
            self.suffix
        );
        let result = sqlx::query(query)
            .bind(self.channel_id)
            .bind(self.guild_id)
            .bind(self.category_id)
            .bind(self.suffix.clone())
            .execute(pool)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::error!(
                    "Failed to insert AutoRoom, CHANNEL({}) GUILD({}) CATEGORY({}) SUFFIX({})\nError: `{}`",
                    self.channel_id,
                    self.guild_id,
                    self.category_id,
                    self.suffix,
                    err
                );
                if let sqlx::Error::Database(db_err) = err {
                    // Unique Contraint error: Postgres "23505", MySQL "1062", SQLite "2067"
                    if db_err.code() == Some("23505".into()) {
                        return Err("Record with given Channel ID already exists")
                    }
                }
                
                Err("Internal Server Error")
            }
        }
    }
    
    pub async fn delete(pool: &PgPool, strategy: AutoRoomDeleteStrategy<'_>) -> Result<(), Error> {
        let query = match strategy {
            AutoRoomDeleteStrategy::SingleByChannelId(id) => sqlx::query("DELETE FROM autoroom WHERE channel_id = $1").bind(id),
            AutoRoomDeleteStrategy::SingleByCategoryId(id) => sqlx::query("DELETE FROM autoroom WHERE category_id = $1").bind(id),
            AutoRoomDeleteStrategy::ManyByChannelId(ids) => sqlx::query("DELETE FROM autoroom WHERE channel_id = ANY($1)",).bind(ids),
            AutoRoomDeleteStrategy::ManyByCategoryId(ids) => sqlx::query("DELETE FROM autoroom WHERE category_id = ANY($1)").bind(ids),
        };

        query
            .execute(pool)
            .await
            .map(|_| ())
    }

    pub async fn get_guild_autorooms(pool: &PgPool, guild_id: i64) -> Result<Vec<Self>, Error> {
        Ok(
            sqlx::query_as::<_, Self>(
                "SELECT * from autoroom WHERE guild_id = $1"
            )
                .bind(guild_id)
                .fetch_all(pool)
                .await?
        )
    }

    pub async fn get_all_category_ids(pool: &PgPool) -> Result<Vec<i64>, Error> {
       Ok(
            sqlx::query_scalar(
                "SELECT category_id from autoroom"
            )
                .fetch_all(pool)
                .await?
        )
    }
}

impl MonitoredAutoRoom {
    pub async fn exists(pool: &PgPool, channel_id: i64) -> bool {
        let query = "SELECT EXISTS(SELECT 1 FROM monitored_autoroom WHERE channel_id = $1)";
        let result = sqlx::query(query)
            .bind(channel_id)
            .fetch_one(pool)
            .await;

        match result {
            Ok(row) => {
                let exists: bool = row.get(0);
                exists
            },
            Err(err) => {
                tracing::error!("{}", err);
                false
            }
        }
    }

    pub async fn remove(pool: &PgPool, channel_id: i64) -> Result<bool, sqlx::Error> {
        let query = "DELETE FROM monitored_autoroom WHERE channel_id = $1";
        let result = sqlx::query(query)
        .bind(channel_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
    
    pub async fn new(pool: &PgPool, channel_id: i64, owner_id: i64) {
        let query = "INSERT INTO monitored_autoroom (channel_id, owner_id) VALUES ($1, $2)";
        sqlx::query(query)
            .bind(channel_id)
            .bind(owner_id)
            .execute(pool)
            .await
            .expect(
                &format!(
                    "Failed to insert MonitoredAutoRoom, CHANNEL({}) OWNER({})",
                    channel_id,
                    owner_id
                )
            );
    }

    pub async fn insert_many(pool: &PgPool, data: &Vec<Self>) -> Result<(), Error> {
        let channel_ids: Vec<i64> = data.iter().map(|a| a.channel_id).collect();
        let owner_ids: Vec<i64> = data.iter().map(|a| a.owner_id).collect();
        sqlx::query(
            r#"
            INSERT INTO monitored_autoroom (channel_id, owner_id)
            SELECT * FROM UNNEST(
                $1::BIGINT[],
                $2::BIGINT[]
            )
            ON CONFLICT (channel_id) DO NOTHING
            "#
        )
            .bind(&channel_ids)
            .bind(&owner_ids)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_by_owner_id(pool: &PgPool, owner_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, Self>("SELECT channel_id, owner_id from monitored_autoroom WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(pool)
            .await {
            Ok(monitored_autoroom) => Ok(Some(monitored_autoroom)),
            Err(err) => match err {
                sqlx::Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<Self>, Error> {
       Ok(
            sqlx::query_as::<_, Self>(
                "SELECT channel_id, owner_id from monitored_autoroom"
            )
                .fetch_all(pool)
                .await?
        )
    }

    pub async fn remove_many(pool: &PgPool, ids: &Vec<i64>) -> Result<(), Error> {
        sqlx::query(
            "DELETE from monitored_autoroom where channel_id = ANY($1)"
        )
        .bind(ids)
        .execute(pool)
        .await
        .map(|_| ())
    }
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct PermamentAutoRoom {
    pub owner_id: i64,
    pub channel_id: i64,
    pub placement_category_id: i64,
    pub storage_category_id: i64
}

impl PermamentAutoRoom {
    
}

pub mod table_builder {
    use async_trait::async_trait;
    use sqlx::{Error, PgPool, postgres::PgQueryResult};
    use super::{ AutoRoom, MonitoredAutoRoom };

    #[async_trait]
    pub trait CreateTable {
        async fn create_table(pool : &PgPool) -> Result<PgQueryResult, Error>;
    }

    #[async_trait]
    impl CreateTable for AutoRoom {
        async fn create_table(pool : &PgPool) -> Result<PgQueryResult, Error> {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS autoroom (
                        channel_id BIGINT PRIMARY KEY,
                        guild_id BIGINT NOT NULL,
                        category_id BIGINT NOT NULL,
                        suffix VARCHAR(16) NOT NULL
                )
                "#,
            )
            .execute(pool)
            .await
        }
    }

    #[async_trait]
    impl CreateTable for MonitoredAutoRoom {
        async fn create_table(pool : &PgPool) -> Result<PgQueryResult, Error> {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS monitored_autoroom (
                        channel_id BIGINT PRIMARY KEY,
                        owner_id BIGINT NOT NULL
                )
                "#,
            )
            .execute(pool)
            .await
        }
    }
}