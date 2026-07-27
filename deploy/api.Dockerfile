# syntax=docker/dockerfile:1
FROM rust:1.96-slim-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY .cargo/config.toml .cargo/config.toml
COPY crates crates
RUN --mount=type=secret,id=cargo,target=/usr/local/cargo/credentials.toml \
    cargo build --release --locked -p api

FROM debian:bookworm-slim AS run
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --home /codehull codehull
WORKDIR /codehull
COPY --from=build /src/target/release/api /usr/local/bin/codehull-api
COPY deploy/keel.toml keel.toml
RUN mkdir -p .local && chown -R codehull:codehull /codehull
USER codehull
EXPOSE 3400
ENTRYPOINT ["codehull-api", "/codehull"]
