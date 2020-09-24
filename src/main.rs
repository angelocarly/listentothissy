use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use std::fs;

use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::{generate_random_string, get_token, process_token, request_token};
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{Args, CommandResult, macros::{
    command,
    group,
}, StandardFramework};
use serenity::model::channel::{Message, PrivateChannel};
use serenity::prelude::{TypeMapKey, TypeMap};
use std::collections::hash_map::RandomState;

use serde_json::json;
use tokio::sync::RwLockWriteGuard;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeStruct;

#[derive(Serialize, Deserialize)]
struct SubscribeData {
    discord_user: u64,
    spotify_user: String,
    spotify_playlist: String
}

#[group]
#[commands(link, follow)]
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
        let cache_file = fs::read_to_string("tokens.json").unwrap_or("".to_string());
        tokens = serde_json::from_str(&*cache_file).expect("Expected correct tokens.json file");
        let subs_file = fs::read_to_string("subs.json");
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

// #[command]
// async fn add(ctx: &Context, msg: &Message) -> CommandResult {
//
//     // Retrieve spotify context
//     let data = ctx.data.read().await;
//     let spotify = data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context");
//
//     let url = msg.content.split_whitespace().nth(1).unwrap_or("");
//     if url.starts_with("https://open.spotify.com/track/") {
//         let track_data = spotify.track(url).await;
//         let track_id = url.split('/').last().unwrap().split('?').next().unwrap();
//         let track_uri = format!("spotify:track:{}", track_id);
//
//         let name = track_data.unwrap().name;
//         println!("Adding {} to playlist", url);
//         let profile = env::var("SPOTIFY_PROFILE").expect("Expected spotify profile");
//         let playlist = env::var("SPOTIFY_PLAYLIST").expect("Expected spotify playlist");
//         let res = spotify.user_playlist_add_tracks(&*profile, &*playlist, &*vec![track_uri], Option::from(0)).await;
//         msg.reply(ctx, format!("Added {:?} to playlist", name)).await?;
//         println!("{:?}", res);
//     }
//
//     Ok(())
// }

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

            // Verify if user owns the playlist
            let spotify_id = spotify.me().await.unwrap().id;
            let playl = spotify.user_playlist(&spotify_id, Some(&mut playlist), None, None).await.unwrap().id;
            if !playlist.ends_with(playl.as_str()) {
                msg.author.dm(&ctx.http, |m| m.content("You do not own this playlist")).await?;
                return Ok(());
            }

            // Insert subscribe data
            let subscribe = SubscribeData {
                discord_user: msg.author.id.0,
                spotify_user: spotify_id,
                spotify_playlist: playl
            };

            let mut map = data.get_mut::<SubscribeContainer>().expect("Expected subscription hashmap");
            map.insert(msg.channel_id.0, subscribe);
            store_cache(data).await;

            msg.author.dm(&ctx.http, |m| m.content("Succesfully followed text channel!")).await?;
        },
        None => {
            msg.author.dm(&ctx.http, |m| m.content("Your account is not linked yet. Message ~link to link it.")).await?;
        }
    }


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
    }
    else {

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

            },
            None => {
                msg.author.dm(&ctx.http, |m| m.content("Failed to link account, did you enter the correct url? Try again by sending ~link")).await?;
            }
        }
    }

    Ok(())
}

async fn store_cache(data: RwLockWriteGuard<'_, TypeMap>) {
    let tokens = serde_json::to_string(data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context")).unwrap();
    let subs = serde_json::to_string(data.get::<SubscribeContainer>().expect("Expected SubscribeContainer in context")).unwrap();
    fs::write("tokens.json", tokens);
    fs::write("subs.json", subs);
}
