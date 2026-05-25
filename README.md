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

---

<!--toc:start-->

- [Configuration](#configuration)
- [Developing with Docker](#developing-with-docker)
  - [Building the Image](#building-the-image)
  - [Running with Docker Compose](#running-with-docker-compose)
  - [Development Workflow](#development-workflow)
- [Production Build](#production-build)
  - [Running the Production Bot](#running-the-production-bot)
- [Limitations and Architecture Notes](#limitations-and-architecture-notes)
<!--toc:end-->

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
docker build -t luna-rs-dev .
```

### Running with Docker Compose

The project includes a `compose.yaml file` to handle the container configuration
and networking. To start the bot, simply run:

```bash
docker compose up --build luna-rs-dev
```

### Development Workflow

If you are actively making code changes and want the bot to recompile
automatically, use the `--watch` flag. This will monitor your local files and
trigger a rebuild whenever you save changes:

```bash
docker compose up --build --watch
```

## Production Build

When you are ready to deploy the bot for actual use, you should build the image
using the `release` profile. This produces a highly optimized, smaller, and
faster binary.

To build the production image, use the `--build-arg` flag from the project root:

```bash
docker build --build-arg BUILD_PROFILE=release -t luna-rs:latest .
```

### Running the Production Bot

The production setup uses two specific services defined in the `compose.yaml` file:

`luna-rs`: The bot service. It uses the optimized production image
(`luna-rs:latest`) built with the `release` profile.

`ytdlp-updater`: A lightweight sidecar container that periodically checks for and
downloads the newest version of `yt-dlp` to a shared volume, ensuring the bot
rarely breaks when YouTube updates its platform.

To start the production bot and the updater in the background, specify their
service names and use the detached (`-d`) flag:

```bash
docker compose up -d luna-rs yt-dlp-updater
```

## Limitations and Architecture Notes

`luna-rs` is strictly designed as a self-hosted, single-instance Discord bot. It
is not built to scale horizontally or run inside a clustered environment (like
Kubernetes pods) due to the following architectural choices:

- In-Memory State Management: The bot tracks active voice sessions, queues, and
  guild (server) states directly in-memory within the application. Because this
  state is not offloaded to a centralized cache (like Redis), running multiple
  instances of the bot would cause fragmented and broken state across different
  servers.

- Inherent Server Affinity: Audio streaming establishes a direct, persistent
  connection between the bot and a guild's voice channel. A single container/pod
  must handle the encoding and streaming pipeline for that specific channel,
  creating a hard server affinity that breaks standard stateless load-balancing
  models.

If you are hosting this for yourself or a few private servers, a single
container deployment is the ideal, low-overhead solution.
