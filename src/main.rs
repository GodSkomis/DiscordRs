use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::prelude::RwLock;
use serenity::prelude::TypeMapKey;
use serenity::model::channel::Message;

use dotenv::dotenv;

use std::env;
use std::collections::HashSet;
use std::sync::Arc;

mod voice;

use voice::{create_proccessing, remove_proccessing, VoiceProccessing};

struct Handler;

struct MonitoredChannels;

impl TypeMapKey for MonitoredChannels {
    type Value = Arc<RwLock<HashSet<ChannelId>>>;
}

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

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

        {
            // Open the data lock in write mode, so keys can be inserted to it.
            let mut data = client.data.write().await;
    
            // The CommandCounter Value has the type: Arc<RwLock<HashMap<String, u64>>>
            // So, we have to insert the same type to it.
            data.insert::<MonitoredChannels>(Arc::new(RwLock::new(HashSet::default())));
        }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
