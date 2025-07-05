use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context as _;
use serenity::{all::VoiceState, async_trait};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
// use tracing::{error, info};

mod voice;
mod bitrate;
pub mod sql;
pub mod commands;
pub mod services;

use sqlx::PgPool;
use voice::{create_proccessing, remove_proccessing, VoiceProccessing};
use sql::{prelude::*, SerenityPool};

use crate::commands::autoroom::savedroom::SavedRoomCache;
use crate::voice::prelude::SavedRoomCacheType;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(response) = VoiceProccessing.proccess(&ctx, &msg).await {
            if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        create_proccessing(&ctx, &new).await;
        if let Some(voice_state) = old {
            remove_proccessing(&ctx, &voice_state).await;
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

    println!("Drop table has begun");
    sqlx::query("DROP table monitored_autoroom").execute(&db).await.expect("Drop table 'monitored_autoroom' unsuccessful");
    println!("Drop table has been completed");

    println!("Table creation has begun");
    create_tables(&db).await;
    println!("Table creation has been completed");

let savedroom_cache: Arc<SavedRoomCache> = Arc::new(Mutex::new(HashMap::new()));

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(commands::generate_commands_framework(
            db.clone(),
            Arc::clone(&savedroom_cache)
        ).await)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<SerenityPool>(db.clone());
        data.insert::<SavedRoomCacheType>(Arc::clone(&savedroom_cache));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(client.into())
}