# syntax=docker/dockerfile:1
FROM debian:bookworm-slim AS run
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --home /codehull codehull
WORKDIR /codehull
COPY deploy/codehull-api /usr/local/bin/codehull-api
COPY deploy/codehull.toml codehull.toml
RUN chmod 0755 /usr/local/bin/codehull-api && mkdir -p .local && chown -R codehull:codehull /codehull
USER codehull
EXPOSE 3400
# The subcommand and its root arrive as args: bootstrap or serve.
ENTRYPOINT ["codehull-api"]
CMD ["serve", "/codehull"]
