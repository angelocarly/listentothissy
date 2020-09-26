use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};

use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::{generate_random_string, get_token, process_token, request_token};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeStruct;
use serde_json::json;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{Args, CommandResult, macros::{
    command,
    group,
    hook,
}, StandardFramework};
use serenity::model::channel::{Message, PrivateChannel};
use serenity::prelude::{TypeMap, TypeMapKey};
use tokio::sync::RwLockWriteGuard;

#[derive(Serialize, Deserialize)]
struct SubscribeData {
    discord_user: u64,
    spotify_user: String,
    spotify_playlist: String,
}


#[group]
#[commands(link, follow, update)]
struct General;

struct Handler;

// A mapping of discord user id's to spotify objects
struct SpotifyContainer;

impl TypeMapKey for SpotifyContainer {
    type Value = HashMap<u64, Spotify>;
}

// A mapping of discord channels to subscribe data
struct SubscribeContainer;

impl TypeMapKey for SubscribeContainer {
    type Value = HashMap<u64, SubscribeData>;
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {

    // Discord setup
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
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

        // Load cache
        let mut tokens = HashMap::new();
        let mut subs = HashMap::new();
        let cache_file = fs::read_to_string("cache/tokens.json");
        if cache_file.is_ok() {
            tokens = serde_json::from_str(&*cache_file.unwrap()).expect("Expected correct tokens.json file");
        }
        let subs_file = fs::read_to_string("cache/subs.json");
        if subs_file.is_ok() {
            subs = serde_json::from_str(&*subs_file.unwrap()).expect("Expected correct subs.json file");
        }

        let mut data = client.data.write().await;
        data.insert::<SpotifyContainer>(tokens);
        data.insert::<SubscribeContainer>(subs);
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[hook]
async fn normal_message(ctx: &Context, msg: &Message) {
    let mut data = ctx.data.write().await;
    let subs = data.get::<SubscribeContainer>().expect("Expected subscription hashmap");

    match subs.get(&msg.channel_id.0) {
        Some(subData) => {
            if let Some(link) = search_spotify_link(&msg.content) {
                match data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context").get(&msg.author.id.0) {
                    Some(spotify) => {
                        // msg.reply(ctx, format!("Added {:?} to playlist", link.clone())).await;
                        add_track(spotify, &subData.spotify_user, &subData.spotify_playlist, link).await;
                    },
                    None => {}
                }
            }
        },
        None => {}
    }
}

// Parse a discord message to find a clean spotify url
fn search_spotify_link(message: &str) -> Option<String> {
    if !message.contains("https://open.spotify.com/track/") {
        return None
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

async fn add_track(spotify: &Spotify, user: &String, playlist: &String, track: String)  {

    println!("{:?}", track);
    if track.starts_with("https://open.spotify.com/track/") {
        let track_data = spotify.track(&*track).await;
        let track_id = track.split('/').last().unwrap().split('?').next().unwrap();
        let track_uri = format!("spotify:track:{}", track_id);
        println!("{:?}", track_uri);

        let name = track_data.unwrap().name;
        let res = spotify.user_playlist_add_tracks(&*user, &*playlist, &*vec![track_uri], Option::from(0)).await;
        println!("{:?}", res);
    }

}

#[command]
async fn follow(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let map = data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context");

    match map.get(msg.author.id.as_u64()) {
        Some(spotify) => {

            // Verify if playlist is the correct format
            let mut playlist = args.message().to_string();
            if !playlist.starts_with("spotify:playlist:") {
                msg.author.dm(&ctx.http, |m| m.content("Your playlist URI is required, this is in the format spotify:playlist:xxxxxxxx")).await?;
                return Ok(());
            }

            // Verify token validity and if user owns the playlist
            match spotify.me().await {
                Ok(user) => {

                    let spotify_id = user.id;
                    let playl = spotify.user_playlist(&spotify_id, Some(&mut playlist), None, None).await.unwrap().id;
                    if !playlist.ends_with(playl.as_str()) {
                        msg.author.dm(&ctx.http, |m| m.content("You do not own this playlist")).await?;
                        return Ok(());
                    }

                    // Insert subscribe data
                    let subscribe = SubscribeData {
                        discord_user: msg.author.id.0,
                        spotify_user: spotify_id,
                        spotify_playlist: playl,
                    };

                    let mut map = data.get_mut::<SubscribeContainer>().expect("Expected subscription hashmap");
                    map.insert(msg.channel_id.0, subscribe);
                    store_cache(data).await;

                    msg.author.dm(&ctx.http, |m| m.content("Succesfully followed text channel!")).await?;

                },
                // User data could not be retrieved
                Err(e) => {
                    msg.author.dm(&ctx.http, |m| m.content(format!("An error occurred:{}", e.to_string()))).await?;
                }
            }
        }
        None => {
            msg.author.dm(&ctx.http, |m| m.content("Your account is not linked yet. Message ~link to link it.")).await?;
        }
    }


    Ok(())
}

/*
 * A test command that refreshes the user's token
 */
#[command]
async fn update(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let mut data = ctx.data.write().await;
    let mut spotifyData = data.get_mut::<SpotifyContainer>().expect("Expected SpotifyContainer in context");

    update_credentials(spotifyData.get_mut(&228235814985924608u64).unwrap()).await;
    store_cache(data).await;

    Ok(())
}
/*
 * Link spotify account to discord id
 */
#[command]
async fn link(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.message().is_empty() {

        // ~link command was empty, check if id is already linked
        let data = ctx.data.read().await;
        let cache = data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context");

        match cache.get(&msg.author.id.0) {
            Some(_) => {
                msg.author.dm(&ctx.http, |m| m.content("Your account is already linked")).await?;
            }
            None => {
                // Craft an authorization url and dm it to the user
                let oauth = SpotifyOAuth::default()
                    .scope("playlist-modify-public")
                    .build();

                let state = generate_random_string(16);
                let auth_url = oauth.get_authorize_url(Some(&state), None);
                let message = format!("In order to link your account, go to the following url, allow access and send me the final url like this: ~link <url>\n{}", auth_url);
                msg.author.dm(&ctx.http, |m| m.content(message)).await?;
            }
        }
    } else {

        // A message was given with the ~link command, try to complete the authorization
        let mut oauth = SpotifyOAuth::default()
            .scope("playlist-modify-public")
            .build();

        let mut url: String = args.message().to_string();
        match process_token(&mut oauth, &mut url).await {
            Some(token_info) => {

                // Building the client credentials with the access token.
                let client_credential = SpotifyClientCredentials::default()
                    .token_info(token_info)
                    .build();

                // Initialize the Spotify client.
                let spotify = Spotify::default()
                    .client_credentials_manager(client_credential)
                    .build();

                let user = spotify.current_user().await.unwrap();
                let welcome_msg = format!("Welcome {}, your account is linked!", user.display_name.unwrap_or(user.id));

                // Store spotify in context
                let mut data = ctx.data.write().await;
                let mut map = data.get_mut::<SpotifyContainer>().expect("Expected spotify hashmap");
                map.insert(msg.author.id.0, spotify);

                store_cache(data).await;
                msg.author.dm(&ctx.http, |m| m.content(welcome_msg)).await?;
                println!("Linked new account, id {}", msg.author.id.0);
            }
            None => {
                msg.author.dm(&ctx.http, |m| m.content("Failed to link account, did you enter the correct url? Try again by sending ~link")).await?;
            }
        }
    }

    Ok(())
}

async fn store_cache(data: RwLockWriteGuard<'_, TypeMap>) {
    let tokens = serde_json::to_string_pretty(data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context")).unwrap();
    let subs = serde_json::to_string_pretty(data.get::<SubscribeContainer>().expect("Expected SubscribeContainer in context")).unwrap();
    fs::write("cache/tokens.json", tokens);
    fs::write("cache/subs.json", subs);
}

async fn verify_credentials(mut spotify: &mut Spotify) -> bool {
    false
}

async fn update_credentials(mut spotify: &mut Spotify) {

    let oauth = SpotifyOAuth::default()
        .scope("playlist-modify-public")
        .build();

    let refresh_token = spotify.client_credentials_manager.as_ref().unwrap().token_info.as_ref().unwrap().refresh_token.as_ref().unwrap();

    let token_info = oauth.refresh_access_token(&*refresh_token).await.unwrap();

    // Building the client credentials, now with the access token.
    let client_credential = SpotifyClientCredentials::default()
        .token_info(token_info)
        .build();

    // Initializing the Spotify client finally.
    spotify = &mut Spotify::default()
        .client_credentials_manager(client_credential)
        .build();
}
