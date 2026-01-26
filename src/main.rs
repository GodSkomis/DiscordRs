use serenity::all::InviteCreateEvent;
use serenity::{all::VoiceState, async_trait};
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, prelude::*};
use tracing_better_stack::{BetterStackLayer, BetterStackConfig};

mod voice;
mod bitrate;
mod sql;
mod commands;
pub mod services;

use voice::{create_proccessing, remove_channel_by_voicestate};
use sql::{prelude::*, SerenityPool};

use crate::services::autoroom::cleanup_categories_monitored_rooms;
use crate::services::autoroom::voice_channel::invite_user;
use crate::sql::pool::SqlPool;
use crate::{services::autoroom::cleanup_db_monitored_rooms, sql::pool::GLOBAL_SQL_POOL};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("`{}` is now online", ready.user.name);
        let mut err = cleanup_db_monitored_rooms(&ctx).await.err();
        if err.is_some() {
            tracing::error!(err);
        };
        err = cleanup_categories_monitored_rooms(&ctx).await.err();
        if err.is_some() {
            tracing::error!(err);
        };
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        create_proccessing(&ctx, &new).await;
        if let Some(voice_state) = old {
            let err = match remove_channel_by_voicestate(&ctx, &voice_state).await {
                Ok(_) => return,
                Err(_err) => _err,
            };
            tracing::error!(err);
        };
    }

    async fn invite_create(&self, ctx: Context, data: InviteCreateEvent) {
        if let Some(guild_id) = data.guild_id {
            if let (Some(author), Some(user)) = (data.inviter, data.target_user) {
                let pool = &GLOBAL_SQL_POOL.get().unwrap().get_pool();
                match invite_user(ctx.http(), pool, author.id.get() as i64, &user).await {
                    Ok(_) => tracing::info!(
                        "Invite event, permissions gived.\nGUILD({}) INVITER({}) TARGET({})",
                        guild_id,
                        author,
                        user
                    ),
                    Err(err) => tracing::error!(
                        "Invite event, give permissions error.\nGUILD({}) INVITER({}) TARGET({})\n{}",
                        guild_id,
                        author,
                        user,
                        err
                    ),
                };
            }
        }
    }
}

fn configurate_logger() {
    let ingesting_host = std::env::var("BETTER_STACK_INGESTING_HOST")
        .expect("BETTER_STACK_INGESTING_HOST must be set");
    let source_token = std::env::var("BETTER_STACK_SOURCE_TOKEN")
        .expect("BETTER_STACK_SOURCE_TOKEN must be set");

    tracing_subscriber::registry()
        .with(BetterStackLayer::new(
            BetterStackConfig::builder(ingesting_host, source_token).build()
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .compact()
        )
        .with(EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .with_env_var("LOG_LEVEL")
            .from_env_lossy()
        )
        .init();
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    configurate_logger();

    let token = std::env::var("DISCORD_TOKEN").unwrap();

    let db_url = std::env::var("POSTGRES_URI").unwrap();
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url).await.unwrap();

    // tracing::info!("Drop table has begun");
    // sqlx::query("DROP table monitored_autoroom").execute(&db).await.expect("Drop table 'monitored_autoroom' unsuccessful");
    // tracing::info!("Drop table has been completed");

    tracing::info!("Table creation has begun");
    if let Err(err) = create_tables(&db).await {
        tracing::error!("Error while creationg sql tables. Finishing...\n Error: `{}`", err);
        return;
    };
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
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_INVITES;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(commands::generate_commands_framework(db.clone()).await)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<SerenityPool>(db.clone());
    };

    tracing::info!("Starting discord bot");
    if let Err(why) = client.start().await {
        tracing::info!("Client error: {:?}", why);
    };
    tracing::info!("Finishing discord bot");

}