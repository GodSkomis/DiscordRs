use sqlx::{Error, Executor, FromRow, PgPool, Postgres};

#[allow(dead_code)]
#[derive(Debug, FromRow, Clone)]
pub struct SavedRoom {
    id: i32,
    pub owner_id: i64,
    pub name: String,
    pub room_name: String,
    pub autoroom_id: i32
}

#[derive(Debug, FromRow)]
pub struct SavedRoomDTO {
    pub owner_id: i64,
    pub name: String,
    pub room_name: String,
    pub autoroom_id: i32
}

impl From<SavedRoom> for SavedRoomDTO {
    fn from(savedroom: SavedRoom) -> Self {
        Self {
            owner_id: savedroom.owner_id,
            name: savedroom.name,
            room_name: savedroom.room_name,
            autoroom_id: savedroom.autoroom_id
        }
    }
}

// #[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct SavedRoomGuest {
    pub savedroom_id: i64,
    pub guest_id: i64,
}

impl SavedRoom {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub async fn get_user_category_savedrooms(pool: &PgPool, owner_id: i64, category_id: i64) -> Result<Vec<Self>, Error> {
        let query = "
            SELECT * FROM savedroom s
            INNER JOIN autoroom a
            ON s.autoroom_id = a.id
            WHERE s.owner_id = $1
                AND a.category_id = $2
            ORDER BY s.id;
        ";
        sqlx::query_as::<_, SavedRoom>(query)
            .bind(owner_id)
            .bind(category_id)
            .fetch_all(pool)
            .await
    }

    // pub async fn insert(&self, pool: &PgPool) { // Make "update" method insted
    //     let query = "INSERT INTO savedroom (guild_id, owner_id, name, autoroom_id) VALUES ($1, $2, $3)";
    //     sqlx::query(query)
    //         .bind(self.owner_id)
    //         .bind(self.room_name.clone())
    //         .bind(self.name.clone())
    //         .bind(self.autoroom_id)
    //         .execute(pool)
    //         .await
    //         .expect(
    //             &format!(
    //                 "Failed to insert SavedRoom, OWNER({}) NAME({}) AUTOROOM({})",
    //                 self.owner_id,
    //                 self.name,
    //                 self.autoroom_id
    //             )
    //         );
    // }

    pub async fn insert(pool: &PgPool, savedroom : &SavedRoomDTO, guests: &Vec<i64>) -> Result<(), Error> {
        let mut tx = pool.begin().await?;
        let query = "INSERT INTO savedroom (owner_id, room_name, name, autoroom_id) VALUES ($1, $2, $3, $4) RETURNING id";
        let savedroom_id: i32 = match sqlx::query_scalar(query)
            .bind(savedroom.owner_id)
            .bind(savedroom.room_name.clone())
            .bind(savedroom.name.clone())
            .bind(savedroom.autoroom_id)
            .fetch_one(&mut *tx)
            .await {
                Ok(_id) => _id,
                Err(err) => {
                    println!(
                        "Failed to insert SavedRoom, OWNER({}) ROOM_NAME({}) NAME({}) AUTOROOM({})",
                        savedroom.owner_id,
                        savedroom.room_name,
                        savedroom.name,
                        savedroom.autoroom_id
                    );
                    return Err(err)
                }
            };

        let _ = SavedRoomGuest::insert_many(&mut *tx, savedroom_id, guests.to_vec()).await?;
        tx.commit().await?;
        Ok(())
    }
    
}


impl SavedRoomGuest {
    pub async fn insert_many(executor: impl Executor<'_, Database = Postgres>, savedroom_id: i32, guest_ids: Vec<i64>) -> Result<(), Error> {
        let query = "INSERT INTO savedroom_guest (savedroom_id, guest_id)
        SELECT * FROM UNNEST($1::integer[], $2::bigint[])";

        sqlx::query(query)
            .bind(vec![savedroom_id; guest_ids.len()])
            .bind(guest_ids)
            .execute(executor)
            .await?;

        Ok(())
    } 
}

mod table_builder {
    use sqlx::PgPool;
    use super::{SavedRoom, SavedRoomGuest};

    impl SavedRoom {
        pub async fn create_table(pool : &PgPool) {
            let _ = sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS savedroom (
                        id SERIAL PRIMARY KEY,
                        owner_id BIGINT UNIQUE NOT NULL,
                        name VARCHAR(16) NOT NULL,
                        room_name VARCHAR(24) NOT NULL,
                        autoroom_id INTEGER NOT NULL REFERENCES autoroom(id) ON DELETE CASCADE
                );
                "#,
            )
            .execute(pool)
            .await;
        }
    }
    impl SavedRoomGuest {
        pub async fn create_table(pool : &PgPool) {
            let _ = sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS savedroom_guest (
                        id SERIAL PRIMARY KEY,
                        guest_id BIGINT NOT NULL,
                        savedroom_id INTEGER NOT NULL REFERENCES savedroom(id) ON DELETE CASCADE
                );
                "#,
            )
            .execute(pool)
            .await;
        }
    }
}