use std::collections::{HashMap, HashSet};
use ::serenity::all::{CreateSelectMenuKind, UserId};
use tokio::sync::Mutex;
use poise::{serenity_prelude as serenity, CreateReply};

use crate::{commands::{CommandContext, CommandError}, services::{autoroom::grant_guest_privileges, utils::get_voice_channel_category}, sql::{prelude::AutoRoom, savedroom::{SavedRoom, SavedRoomDTO, SavedRoomGuest}}};
use crate::services::utils::get_user_guild_voice_channel;


#[derive(Debug, Clone)]
pub struct SaveRoomCacheRecord {
    owner_id: i64,
    guests: HashSet<i64>
}

impl SaveRoomCacheRecord {
    pub fn new(owner_id: i64, guest: Option<i64>) -> Self {
        let mut record = Self {
            owner_id: owner_id,
            guests: HashSet::new()
        };
        if let Some(guest_id) = guest {
            record.guests.insert(guest_id);
        };
        record
    }
}

pub type SavedRoomCache = Mutex<HashMap<i64, SaveRoomCacheRecord>>;


#[poise::command(slash_command)]
pub async fn save(
    ctx: CommandContext<'_>,
    #[description = "Name for a record"] name: Option<String>,
) -> Result<(), CommandError> {

    let voice_channel = get_user_guild_voice_channel(&ctx, None).await?;

    let room_cache = {
        let cache = &ctx.data().savedroom_cache.lock().await;
        let record = match cache.get(&(voice_channel.id.get() as i64)) {
            Some(room_cache) => room_cache.clone(),
            None => return Err(CommandError::from("Nothing to save"))
        };
        record
    };

    if room_cache.guests.is_empty() {
        return Err(CommandError::from("Nothing to save"))
    }

    let category_id = get_voice_channel_category(&voice_channel)?;

    let pool = &ctx.data().pool;
    let autoroom = match AutoRoom::get_by_category_id(pool, category_id.get() as i64).await {
        Ok(Some(_autoroom)) => _autoroom,
        Ok(None) => return Err(CommandError::from(
            format!(
                "This room is outside autoroom category, CategoryID: {}",
                &category_id
            )
        )),
        Err(err) => {
            println!("{:?}", err);
            return Err(CommandError::from("Something go wrong, please try again later"))
        }
    };
    
    let savedroom = SavedRoomDTO {
        autoroom_id: autoroom.id(),
        owner_id: room_cache.owner_id,
        name: match name {
            Some(_name) => _name,
            None => voice_channel.name.clone()
        },
        room_name: voice_channel.name.clone(),
    };
    let _ = match SavedRoom::insert(pool, &savedroom, &Vec::from_iter(room_cache.guests)).await {
        Ok(_) => ctx.say(
            format!(
                "The room record '{}' has been successfully saved",
                savedroom.name
            )
        ).await?,
        Err(err) => {
            println!("{:?}", err);
            return Err(CommandError::from("Something go wrong, please try again later"))
        }
    };

    Ok(())
}


#[derive(Debug, poise::Modal)]
#[name = "A room record name"]
#[allow(dead_code)]
struct SavedRoomModal {
    #[name = "Record name"]
    #[placeholder = "Choose a name to load"]
    record_name: String
}


#[poise::command(slash_command)]
pub async fn load(
    ctx: CommandContext<'_>
) -> Result<(), CommandError> {

    let user_id = ctx.author().id;
    let voice_channel = get_user_guild_voice_channel(&ctx, Some(&user_id)).await?;
    let category_id = get_voice_channel_category(&voice_channel)?;
    let pool = &ctx.data().pool;
    let savedrooms = match SavedRoom::get_user_category_savedrooms(
        pool, ctx.author().id.get() as i64,
        category_id.get() as i64
    ).await {
        Ok(rows) => {
            if rows.is_empty() {
                return Err(CommandError::from("Something go wrong, please try again later"))
            };
            rows
        },
        Err(err) => {
            println!("Failed to load user`s: {:?} savedrooms.\n{:?}", user_id.get(), err);
            return Err(CommandError::from("Something go wrong, please try again later"))
        }
    };

    let options: Vec<serenity::CreateSelectMenuOption> = savedrooms.clone()
        .into_iter()
        .map(|_savedroom| {
            let option = serenity::CreateSelectMenuOption::new(
                _savedroom.name.clone(),
                _savedroom.name.clone()
            );
            let option = option.description(format!("Room name: {}", _savedroom.room_name.clone()));
            option
        })
        .collect();
    
    let custom_modal_id = "savedroom_select";
    let select_menu = serenity::CreateSelectMenu::new(
        custom_modal_id,
        CreateSelectMenuKind::String { options: options }
    );

    let action_row = serenity::CreateActionRow::SelectMenu(select_menu);

    let reply = ctx.send(
        CreateReply::default()
            .content("Choose a record to load: ")
            .components(vec![action_row]),
    )
    .await?;

    let msg_ref = reply.message().await?;

    // Ждём взаимодействия
    if let Some(interaction) = msg_ref
        .await_component_interaction(ctx.serenity_context())
        .author_id(user_id)
        .timeout(std::time::Duration::from_secs(30))
        .await
    {
        // Обработка выбора
        if let serenity::ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
            let record_name = values.get(0).unwrap().clone();
            let profile = match savedrooms.iter().find(|&room| room.name == record_name) {
                Some(_room) => _room.clone(),
                None => return Err(CommandError::from("Profile with given name not found.")),
            };

            // interaction
            //     .create_response(ctx.serenity_context(), serenity::CreateInteractionResponse::UpdateMessage(
            //         serenity::CreateInteractionResponseMessage::new()
            //             .content(format!("🎉Profile: `{}`", profile.name))
            //     ))
            //     .await?; // Don't delete this block. This is actually right way to wrok with interactions
            
            interaction.create_response(ctx.serenity_context(), serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!("⏳ Loading profile: '{}', please wait and don't change room settings to avoid overwriting ⏳", record_name))
                    .components(vec![])
            )).await?;

            let guests = match SavedRoomGuest::get_guests(pool, profile.id()).await {
                Ok(_guests) => _guests,
                Err(err) => {
                    println!("Failed to load guests of savedroom {:?}.\n{:?}", profile, err);
                    return Err(CommandError::from("Something go wrong, please try again later"))
                }
            };
            let mut mentions: Vec<String> = Vec::new();
            for guest in guests {
                let guest_id = UserId::new(guest.guest_id as u64);
                if let Err(err) = grant_guest_privileges(&ctx, &voice_channel.id, &guest_id).await {
                    eprintln!("{:?}", err);
                    continue;
                };
                mentions.push(format!("<@{}>", guest.guest_id as u64));
            };
            interaction.create_response(ctx.serenity_context(), serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!("✅Pofile: '{}' have been successfully loaded✅", record_name))
                    .components(vec![])
            )).await?;
            ctx.say(mentions.join(" ")).await?;

        } else {
            eprintln!("Wrong component kind");
            return Err(CommandError::from("Something go wrong, please try again later"))
        }
    } else {
        ctx.say("Timeout").await?;
    }

    Ok(())

}




    // let err_msg = format!("The connected voice channel was not found");
    // let monitored_autoroom = match MonitoredAutoRoom::get_by_owner_id(pool, author.id.get() as i64).await {
    //     Ok(option) => match option {
    //         Some(monitored_autoroom_result) => monitored_autoroom_result,
    //         None => return Err(CommandError::from(err_msg))
    //     },
    //     Err(_) => return Err(CommandError::from(err_msg)),
    // };

    // let channel_id = ChannelId::new(monitored_autoroom.channel_id as u64);
    // let result_msg = match grant_guest_privileges(&ctx, &channel_id, &user.id).await {
    //     Ok(_) => &format!(
    //         "{} has been successfully invited",
    //         user.mention().to_string()
    //     ),
    //     Err(_) => &format!(
    //         "Failed to invite {:?}",
    //         user.name
    //     ),
    // };
    
    // ctx.say(result_msg).await?;
    // Ok(())