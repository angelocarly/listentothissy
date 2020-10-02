use std::collections::HashMap;
use std::env;
use std::fs;

use rspotify::client::Spotify;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::client::{Client, Context};
use serenity::framework::standard::{CommandResult, macros::{
    command,
    group,
    hook,
}, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::{EventHandler, TypeMapKey};

use commands::follow::*;
use commands::link::*;

use crate::util::{check_update_token, get_refresh_credentials, is_valid_token, update_cache};
use std::time::UNIX_EPOCH;

mod commands;
mod util;

#[derive(Serialize, Deserialize)]
struct SubscribeData {
    discord_user: u64,
    spotify_user: String,
    spotify_playlist: String,
}

#[derive(Serialize, Deserialize)]
pub struct ThissyData {
    subdata: HashMap<u64, SubscribeData>,
    spotify_map: HashMap<u64, Spotify>,
}

#[group]
#[commands(link, update, follow)]
struct General;

struct Handler;

// A mapping of discord user id's to spotify objects
struct ThissyContainer;

impl TypeMapKey for ThissyContainer {
    type Value = ThissyData;
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {


    // Discord setup
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot prefix to "~"
        .normal_message(normal_message)
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::new(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    {

        let mut data = client.data.write().await;

        // Load cache
        let mut thissy_data;
        let cache_string = fs::read_to_string("cache/cache.json");
        if !cache_string.is_err() {
            thissy_data = serde_json::from_str::<ThissyData>(cache_string.unwrap().as_str()).expect("Couldn't parse cache");
        } else {
            thissy_data = ThissyData {
                spotify_map: HashMap::new(),
                subdata: HashMap::new()
            }
        }
        data.insert::<ThissyContainer>(thissy_data);
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}


#[hook]
/*
 *  Message hook
 *  Each message that is not a command will pass through this function in order to find spotify urls
 */
async fn normal_message(ctx: &Context, msg: &Message) {

    let mut updated_cache = false;

    // Obtain context data
    let mut data = ctx.data.write().await;
    if let Some(thissy_data) = data.get_mut::<ThissyContainer>() {

        // Search for current channel entries
        match thissy_data.subdata.get(&msg.channel_id.0) {
            Some(sub_data) => {

                // Search for a spotify link in the message
                if let Some(link) = search_spotify_link(&msg.content) {

                    // Obtain a reference to the user subscribed to this channel
                    if let Some(spotify) = thissy_data.spotify_map.get_mut(&msg.author.id.0) {

                        // Verify token validity and refresh
                        if check_update_token(spotify).await {
                            updated_cache = true;
                        }

                        // Add the track to the playlist
                        msg.reply(ctx, format!("Added {:?} to playlist", link.clone())).await;
                        add_track(spotify, &sub_data.spotify_user, &sub_data.spotify_playlist, link).await;
                    }
                }
            }
            None => {}
        }
    }

    if updated_cache {
        if let Some(thissy_data) = data.get::<ThissyContainer>() {
            update_cache(thissy_data);
        }
    }
}

// Parse a discord message to find a clean spotify url
fn search_spotify_link(message: &str) -> Option<String> {
    if !message.contains("https://open.spotify.com/track/") {
        return None;
    }

    // Ignore strings starting with '>'
    for i in message.split('\n') {
        if !i.starts_with('>') {
            for j in i.split(' ') {
                if j.starts_with("https://open.spotify.com/track/") {
                    return Some(j.to_string());
                }
            }
        }
    }
    None
}

/*
 *  Adds a track to a playlist
 *  @param spotify, a valid spotify object
 */
async fn add_track(mut spotify: &mut Spotify, user: &String, playlist: &String, track: String) {
    println!("Adding track {:?} to playlist {}", track, playlist);
    if track.starts_with("https://open.spotify.com/track/") {


        // Add the track to the given playlist
        let track_data = spotify.track(&*track).await;
        let track_id = track.split('/').last().unwrap().split('?').next().unwrap();
        let track_uri = format!("spotify:track:{}", track_id);
        println!("{:?}", track_uri);

        let _name = track_data.unwrap().name;
        match spotify.user_playlist_add_tracks(&*user, &*playlist, &*vec![track_uri], Option::from(0)).await {
            Ok(_) => {
                println!("Successfully added track");
            }
            Err(err) => {
                println!("Something went wrong adding the track {}", err.to_string());
            }
        }
    }
}


/*
 * A test command that refreshes the user's token
 */
#[command]
async fn update(ctx: &Context) -> CommandResult {
    let mut data = ctx.data.write().await;
    // let spotify_data = data.get_mut::<SpotifyContainer>().expect("Expected SpotifyContainer in context");

    // get_refresh_credentials(spotify_data.get_mut(&228235814985924608u64).unwrap()).await;
    // store_cache(data).await;

    Ok(())
}

