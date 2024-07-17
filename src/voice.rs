use serenity::model::voice::VoiceState;
use serenity::model::id::ChannelId;
use serenity::builder::{ CreateChannel };
use serenity::client::Context;
use serenity::model::channel::{ Message, Channel };

use super::{ MonitoredChannels };

pub async fn create_proccessing(ctx: &Context, new: &VoiceState) {
    if let Some(channel_id) = new.channel_id {
        // Убедитесь, что это нужный вам канал, для которого нужно создать новый
        if channel_id == ChannelId::new(1262847960392400976 as u64) {
            if let Some(guild_id) = new.guild_id {
                if let Some(member) = &new.member {
                    let user_id = member.user.id;
                    let user_name = &member.user.name;

                    // Создаем новый голосовой канал с именем пользователя
                    let builder = CreateChannel::new(user_name)
                        .category(ChannelId::new(946552116548362301 as u64))
                            .kind(serenity::model::channel::ChannelType::Voice);
                    let channel_result = guild_id.create_channel(&ctx.http, builder).await;

                    if let Ok(channel) = channel_result {
                        // Переносим пользователя в созданный канал
                        if let Err(why) = guild_id.move_member(&ctx.http, user_id, channel.id).await {
                            println!("Error moving user: {:?}", why);
                        }
                        let monitored_channels_lock = {
                            let data_read = ctx.data.read();
                    
                            // Since the CommandCounter Value is wrapped in an Arc, cloning will not duplicate the
                            // data, instead the reference is cloned.
                            // We wrap every value on in an Arc, as to keep the data lock open for the least time
                            // possible, to again, avoid deadlocking it.
                            data_read.await.get::<MonitoredChannels>().expect("Expected MonitoredChannels in TypeMap.").clone()
                        };
                        {
                            let mut monitored_channels = monitored_channels_lock.write().await;
                            monitored_channels.insert(channel.id);
                        };
                    }
                }
            }
        }
    }
}

pub async fn remove_proccessing(ctx: &Context, new: &VoiceState) {
    if let Some(channel_id) = &new.channel_id {
        let monitored_channels_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<MonitoredChannels>().expect("Expected MonitoredChannels in TypeMap.").clone()
        };
        let is_monitored: bool = {
            let monitored_channels = monitored_channels_lock.read().await;
            monitored_channels.contains(&channel_id)
        };
        
        if is_monitored == true {
            match channel_id.to_channel(&ctx.http).await {
                Ok(channel) => {
                    // if let Some(members) = &channel.members(&ctx.http).await.unwrap() {
                    match &channel.clone().guild().unwrap().members(&ctx.cache) {
                        Ok(members) => {
                            if members.len() == 0 {
                                let _ = channel.delete(&ctx.http).await;

                                let monitored_channels_lock = {
                                    let data_read = ctx.data.read();
                            
                                    // Since the CommandCounter Value is wrapped in an Arc, cloning will not duplicate the
                                    // data, instead the reference is cloned.
                                    // We wrap every value on in an Arc, as to keep the data lock open for the least time
                                    // possible, to again, avoid deadlocking it.
                                    data_read.await.get::<MonitoredChannels>().expect("Expected MonitoredChannels in TypeMap.").clone()
                                };
                                {
                                    let mut monitored_channels = monitored_channels_lock.write().await;
                                    monitored_channels.remove(channel_id);
                                };
                            };
                        },
                    Err(err) => println!("Err: {}", err)
            }},
                Err(err) => println!("Err: {}", err)
            }
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
        println!("Channel id: {}, Category id: {}", channel_id, category_id);
        return format!("Channel id: {}, Category id: {}", channel_id, category_id);
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