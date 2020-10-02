use serenity::prelude::TypeMap;
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyOAuth, SpotifyClientCredentials};
use std::fs;
use crate::ThissyData;
use std::error::Error;

pub fn update_cache(thissy_data: &ThissyData) {
    let thissy_string = serde_json::to_string_pretty(thissy_data).expect("Failed to parse thissy_data");
    fs::write("cache/cache.json", thissy_string).expect("Wasn't able to write to cache file");
}

/*
 * Check and update a spotify token, if invalid update it
 * @returns whether token is updated
 */
pub async fn check_update_token(mut spotify: &mut Spotify) -> bool {
    let valid = is_valid_token(spotify).await;

    if !valid {
        spotify = &mut get_refresh_credentials(spotify).await;
        return true;
    }
    false
}

pub async fn is_valid_token(spotify: &Spotify) -> bool {
    false
}

pub async fn get_refresh_credentials(spotify: &Spotify) -> Spotify {

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
    return Spotify::default()
        .client_credentials_manager(client_credential)
        .build();
}
