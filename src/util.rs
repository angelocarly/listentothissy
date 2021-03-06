use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyOAuth, SpotifyClientCredentials};
use std::fs;
use crate::ThissyData;
use std::time::UNIX_EPOCH;

pub fn update_cache(thissy_data: &ThissyData) {
    let thissy_string = serde_json::to_string_pretty(thissy_data).expect("Failed to parse thissy_data");
    fs::write("cache/cache.json", thissy_string).expect("Wasn't able to write to cache file");
}

pub fn is_valid_token(spotify: &Spotify) -> bool {

    let unixtime = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let credential = spotify.client_credentials_manager.clone().unwrap();
    return ((unixtime + 10) as i64) < credential.token_info.unwrap().expires_at.unwrap();
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

pub fn create_spotify_playlist_link(uri: &str) -> String {
    return "https://open.spotify.com/playlist/".to_string() + uri;
}

