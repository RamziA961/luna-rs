# luna-rs

![luna-rs logo](./assets/profile.png)

luna-rs is a Discord bot built to handle audio playback simply and reliably.
It’s written in Rust and designed handle your music queue and plays content
directly in your voice channels.

I started this project because I was frustrated with the state of public music
bots. Many of them have become increasingly unreliable due to YouTube’s
evolving restrictions on third-party API access, and those that do work often
prioritize server-side cost savings by forcing low audio bitrates. luna-rs aims
to solve both by providing a stable, self-hosted alternative that prioritizes
high-fidelity audio output.

It does what it says on the tin—nothing more, nothing less.

## Configuration

To run the bot, you will need a Discord Bot Token and a YouTube API Key. These
should be stored in a file named `Secrets.toml` located at the root of the
project directory.

Create the file and add your keys as follows:

```toml
# Secrets.toml
DISCORD_TOKEN = "your_discord_bot_token_here"
YOUTUBE_API_KEY = "your_youtube_api_key_here"
```

## Developing with Docker

You don't need to install the Rust toolchain locally if you prefer using Docker.
This is the recommended way to build and run the bot in an isolated environment.

### Building the Image

To build the image locally, run the following command from the project root:

```bash
docker build -t luna-rs .
```

### Running with Docker Compose

The project includes a `compose.yaml file` to handle the container configuration
and networking. To start the bot, simply run:

```bash
docker compose up --build
```

### Development Workflow

If you are actively making code changes and want the bot to recompile
automatically, use the `--watch` flag. This will monitor your local files and
trigger a rebuild whenever you save changes:

```bash
docker compose up --build --watch
```
