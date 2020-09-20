use std::env;
use std::sync::{Arc, Mutex};

use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::get_token;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    CommandResult,
    macros::{
        command,
        group,
    },
    StandardFramework,
};
use serenity::model::channel::Message;
use serenity::prelude::TypeMapKey;

#[group]
#[commands(add, sync)]
struct General;

struct Handler;

struct SpotifyContainer;

impl TypeMapKey for SpotifyContainer {
    type Value = Arc<Spotify>;
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    // Spotify setup
    let spotify;
    let mut oauth = SpotifyOAuth::default()
        .scope("playlist-modify-public")
        .build();
    match get_token(&mut oauth).await {
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .build();
            spotify = Spotify::default()
                .client_credentials_manager(client_credential)
                .build();
            println!("Auth succeeded");
        }
        None => panic!("auth failed"),
    };

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
        let mut data = client.data.write().await;
        data.insert::<SpotifyContainer>(Arc::new(spotify));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn add(ctx: &Context, msg: &Message) -> CommandResult {

    // Retrieve spotify context
    let data = ctx.data.read().await;
    let spotify = data.get::<SpotifyContainer>().expect("Expected SpotifyContainer in context");

    let url = msg.content.split_whitespace().nth(1).unwrap_or("");
    if url.starts_with("https://open.spotify.com/track/") {
        let track_data = spotify.track(url).await;
        let track_id = url.split('/').last().unwrap().split('?').next().unwrap();
        let track_uri = format!("spotify:track:{}", track_id);

        let name = track_data.unwrap().name;
        println!("Adding {} to playlist", url);
        let res = spotify.user_playlist_add_tracks("31yxo4oikta6ebumsj26ivlanwji", "0j2PpLzQD4d1y3ZkeJFVqa", &*vec![track_uri], Option::from(0)).await;
        msg.reply(ctx, format!("Added {:?} to playlist", name)).await?;
        println!("{:?}", res);
    }

    Ok(())
}

#[command]
async fn sync(ctx: &Context, msg: &Message) -> CommandResult {

    println!("OK");

        Ok(())
}
