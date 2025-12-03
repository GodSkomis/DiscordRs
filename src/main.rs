use anyhow::Context as _;
use serenity::{all::VoiceState, async_trait};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;

mod voice;
mod bitrate;
mod sql;
mod commands;
pub mod services;

use sqlx::PgPool;
use voice::{create_proccessing, remove_channel_by_voicestate, VoiceProccessing};
use sql::{prelude::*, SerenityPool};

use crate::sql::pool::SqlPool;
use crate::{services::autoroom::cleanup_db_monitored_rooms, sql::pool::GLOBAL_SQL_POOL};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("`{}` is now online", ready.user.name);
        let err = cleanup_db_monitored_rooms(&ctx).await.err();
        if err.is_some() {
            tracing::error!(err);
        };
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(response) = VoiceProccessing.proccess(&ctx, &msg).await {
            if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                tracing::error!("Error sending message: {why:?}");
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        create_proccessing(&ctx, &new).await;
        if let Some(voice_state) = old {
            let err = remove_channel_by_voicestate(&ctx, &voice_state).await.unwrap_err();
            tracing::error!(err);
        };
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
    #[shuttle_shared_db::Postgres] db: PgPool
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    // tracing::info!("Drop table has begun");
    // sqlx::query("DROP table monitored_autoroom").execute(&db).await.expect("Drop table 'monitored_autoroom' unsuccessful");
    // tracing::info!("Drop table has been completed");

    tracing::info!("Table creation has begun");
    create_tables(&db).await;
    tracing::info!("Table creation has been completed");

    match GLOBAL_SQL_POOL.set(SqlPool::new(db.clone())) {
        Ok(_) => (),
        Err(_) => {
            tracing::error!("GLOBAL_SQL_POOL isn't empty");
            GLOBAL_SQL_POOL.get().expect("Unable to get GLOBAL_SQL_POOL");
        },
    };


    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(commands::generate_commands_framework(db.clone()).await)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<SerenityPool>(db.clone());
    }

    if let Err(why) = client.start().await {
        tracing::info!("Client error: {:?}", why);
    }

    Ok(client.into())
}