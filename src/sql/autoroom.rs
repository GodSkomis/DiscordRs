use sqlx::{Error, FromRow, PgPool, Row};


#[derive(Debug, FromRow)]
pub struct AutoRoom {
    pub channel_id: i64,
    pub category_id: i64,
    pub suffix: String
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AutoRoomDeleteStrategy {
    SingleByChannelId(i64),
    SingleByCategoryId(i64),
    ManyByChannelId(Vec<i64>),
    ManyByCategoryId(Vec<i64>)
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct MonitoredAutoRoom {
    pub channel_id: i64,
    pub owner_id: i64
}

impl AutoRoom {
    pub async fn get_by_channel_id(pool: &PgPool, channel_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, AutoRoom>("SELECT channel_id, category_id, suffix from autoroom WHERE channel_id = $1")
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

    pub async fn create(&self, pool: &PgPool) {
        let query = "INSERT INTO autoroom (channel_id, category_id, suffix) VALUES ($1, $2, $3)";
        sqlx::query(query)
            .bind(self.channel_id)
            .bind(self.category_id)
            .bind(self.suffix.clone())
            .execute(pool)
            .await
            .expect(
                &format!(
                    "Failed to insert AutoRoom, CHANNEL({}) CATEGORY({}) SUFFIX({})",
                    self.channel_id,
                    self.category_id,
                    self.suffix
                )
            );
    }
    
    pub async fn delete(pool: &PgPool, strategy: AutoRoomDeleteStrategy) -> Result<(), Error> {
        let query = match strategy {
            AutoRoomDeleteStrategy::SingleByChannelId(id) => sqlx::query("DELETE FROM autoroom WHERE channel_id = $1").bind(id),
            AutoRoomDeleteStrategy::SingleByCategoryId(id) => sqlx::query("DELETE FROM autoroom WHERE category_id = $1").bind(id),
            AutoRoomDeleteStrategy::ManyByChannelId(ids) => sqlx::query("DELETE FROM autoroom WHERE channel_id in $1",).bind(ids),
            AutoRoomDeleteStrategy::ManyByCategoryId(ids) => sqlx::query("DELETE FROM autoroom WHERE category_id in $1").bind(ids),
        };

        query
            .execute(pool)
            .await
            .map(|_| ())
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

    pub async fn remove_many(pool: &PgPool, ids: Vec<i64>) -> Result<(), Error> {
        sqlx::query(
            "DELETE from monitored_autoroom where channel_id in $1"
        )
        .bind(ids)
        .execute(pool)
        .await
        .map(|_| ())
    }
}

mod table_builder {
    use sqlx::PgPool;
    use super::{AutoRoom, MonitoredAutoRoom};

    impl AutoRoom {
        pub async fn create_table(pool : &PgPool) {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS autoroom (
                        id SERIAL PRIMARY KEY,
                        channel_id BIGINT UNIQUE NOT NULL,
                        category_id BIGINT NOT NULL,
                        suffix VARCHAR(16) NOT NULL
                )
                "#,
            )
            .execute(pool)
            .await
            .expect("Failed to create autoroom table");
        }
    }
    impl MonitoredAutoRoom {
        pub async fn create_table(pool : &PgPool) {
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
            .expect("Failed to create autoroom table");
        }
    }
}