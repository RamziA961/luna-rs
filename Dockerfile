ARG BUILD_PROFILE="release"

# Stage 1: Build the binary
FROM rust:1.95-alpine AS builder
ARG BUILD_PROFILE
ARG TARGETARCH

WORKDIR /volume
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static cmake make g++ gcc

# Cache dependencies 
RUN mkdir src && echo "fn main() {}" > src/main.rs
COPY Cargo.toml Cargo.lock ./

RUN if [ "$BUILD_PROFILE" = "release" ]; then \
  cargo build --release --target-dir /volume/target/output; \
  else \
  cargo build --target-dir /volume/target/output; \
  fi

# Copy source code
COPY src ./src

RUN touch src/main.rs && \
  if [ "$BUILD_PROFILE" = "release" ]; then \
  cargo build --release --target-dir /volume/target/output; \
  else \
  cargo build --target-dir /volume/target/output; \
  fi


# Stage 2: ultra-slim runtime image
FROM alpine:3.19
ARG BUILD_PROFILE
WORKDIR /app

SHELL ["/bin/ash", "-o", "pipefail", "-c"]

RUN apk add --no-cache \
  ca-certificates \
  openssl \
  ffmpeg \
  python3 \
  curl \
  unzip \
  bash

RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
  && chmod a+rx /usr/local/bin/yt-dlp

RUN yt-dlp --version && bun --version

RUN mkdir -p /etc/yt-dlp \
  && echo "--js-runtime bun" > /etc/yt-dlp/config

COPY --from=builder /volume/target/output/${BUILD_PROFILE}/luna-rs .

CMD ["./luna-rs"]
