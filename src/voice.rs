use serenity::model::voice::VoiceState;
use serenity::model::id::ChannelId;
use serenity::builder::{ CreateChannel };
use serenity::client::Context;
use serenity::model::channel::{ Message, Channel };

use super::{ MonitoredChannels, MONITORED_STR_VALUE };
use super::bitrate::get_bitrate;


// const CHANNEL_ID: u64 = 1263582863413088266;

fn _check_create_permissions(v: &Vec<&str>, value: &str) -> bool {
    if v.len() < 2 {
        return false;
    }
    if v[0] == value {
        return true;
    }
    false
}

pub async fn create_proccessing(ctx: &Context, new: &VoiceState) {
    if let Some(channel_id) = new.channel_id {
        let is_monitored:bool = match MonitoredChannels.get(&channel_id.to_string()).await {
            Ok(result) => match result {
                    Some(string) => {
                        let words: Vec<&str> = string.split(' ').collect();
                        _check_create_permissions(&words, &channel_id.to_string())
                    }
                    None => false
                },
            Err(_) => return
        };
        if is_monitored == false {
            return;
        }
        if let Some(guild_id) = new.guild_id {
            // Get max available server bitrate
            let max_bitrate = get_bitrate(&guild_id.to_guild_cached(&ctx.cache).unwrap().premium_tier);

            if let Some(member) = &new.member {
                let user_id = member.user.id;
                let user_name = &member.user.name;

                // Создаем новый голосовой канал с именем пользователя
                let builder = CreateChannel::new(user_name)
                    .category(ChannelId::new(946552116548362301 as u64))
                        .kind(serenity::model::channel::ChannelType::Voice)
                            .bitrate(max_bitrate);
                let channel_result = guild_id.create_channel(&ctx.http, builder).await;

                if let Ok(channel) = channel_result {
                    // Переносим пользователя в созданный канал
                    if let Err(why) = guild_id.move_member(&ctx.http, user_id, channel.id).await {
                        println!("Error moving user: {:?}", why);
                        return;
                    }

                    let _  = MonitoredChannels.set(&channel.id.get().to_string(), bytes::Bytes::copy_from_slice(MONITORED_STR_VALUE.as_bytes()), Some(24 * 60 * 60)).await;
                    
                }
            }
        }
    }
}

pub async fn remove_proccessing(ctx: &Context, new: &VoiceState) {
    if let Some(channel_id) = &new.channel_id {
        let is_monitored = match MonitoredChannels.get(&channel_id.get().to_string()).await {
            Ok(result) => match result {
                Some(string) => string == MONITORED_STR_VALUE,
                None => false
        },
            Err(err) => { println!("Err: {}", err); return;}
        };
        
        println!("DELETE?: {}, channel: {}", is_monitored, channel_id);
        if is_monitored == false {
            return;
        }

        match channel_id.to_channel(&ctx.http).await {
            Ok(channel) => {
                // if let Some(members) = &channel.members(&ctx.http).await.unwrap() {
                match &channel.clone().guild().unwrap().members(&ctx.cache) {
                    Ok(members) => {
                        if members.len() == 0 {
                            let _ = channel.delete(&ctx.http).await;
                        };
                    },
                Err(err) => println!("Err: {}", err)
        }},
            Err(err) => println!("Err: {}", err)
        }
    }
}

pub struct VoiceProccessing;

impl VoiceProccessing {
    pub async fn proccess(&self, ctx: &Context, msg: &Message) -> Option<String>{
        if !msg.content.starts_with("!autoroom") {
            return None;
        };
        let commands: Vec<&str> = msg.content.split(' ').collect();
        if commands.len() < 4 {
            return Some(format!("Wrong number of arguments: {}", commands.len()));
        };
        match commands[1] {
            "add" => return Some(self.proccess_add(ctx, msg, commands).await),
            "list" => return Some(String::from("NotImplemented")),
            _ => return Some(String::from("Wrong subcommand"))
        };
    }

    async fn proccess_add(&self, ctx: &Context, msg: &Message, commands: Vec<&str>) -> String {
        let channel_id = ChannelId::new(commands[2].parse::<u64>().unwrap());
        let category_id = ChannelId::new(commands[3].parse::<u64>().unwrap());
        if !self.check_channel(ctx, msg, &channel_id).await {
            return format!("Wrong channel id: {}", channel_id);
        }
        if !self.check_channel(ctx, msg, &category_id).await {
            return format!("Wrong category id: {}", category_id);
        }
        let data = format!("{} {}", channel_id, category_id);
        MonitoredChannels.set(&channel_id.get().to_string(), bytes::Bytes::copy_from_slice(data.as_bytes()), None).await;
        return format!("Record was created! channel id: {}, category id: {}", channel_id, category_id);
    }

    async fn check_channel(&self, ctx: &Context, msg: &Message, channel_id: &ChannelId) -> bool {
        if let Some(channel) = self.get_channel(ctx, channel_id).await {
            if let Some(guild_channel) = channel.guild() {
                if let Some(guild_id) = msg.guild_id {
                    return guild_channel.guild_id == guild_id;
                }
            }
        }
        return false;
    }

    async fn get_channel(&self, ctx: &Context, channel_id: &ChannelId) -> Option<Channel> {
    if let Ok(channel) = channel_id.to_channel(&ctx.http).await {
        return Some(channel);
    };
    None
}
}