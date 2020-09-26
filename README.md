# listentothissy
Discord bot that creates spotify playlists based on discord messages.

**What does it do**:  
Listentothissy is a bot that's able to link a spotify playlist and a discord channel together. Whenever a spotify track is posted in the channel, the track is also added to the playlist.

## Configuration
This bot requires a few necessary environment variables in order to function. These are;
```
- DISCORD_TOKEN - Discord bot token
- CLIENT_ID - spotify client id
- CLIENT_SECRET - spotify client secret
- REDIRECT_URI - spotify redirect uri
```

## Usage

### Link account
- Get an authorization url by sending `~link`
- Visit the url and authorize the bot
- Send the following url to the bot using `~link <url>`

## Running

You can run this bot using cargo:

```
```

Docker-compose example:
```
version: '3'

services:
  thissy:
    build:
      context: ./listentothissy
      dockerfile: Dockerfile
    container_name: thissy
    volumes:
      - ./cache:/usr/src/thissy/cache
    environment:
      DISCORD_TOKEN: token
      CLIENT_ID: id
      CLIENT_SECRET: secret
      REDIRECT_URI: http://url:port
```
