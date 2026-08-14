# syntax=docker/dockerfile:1
FROM debian:bookworm-slim AS run
# git is a runtime dependency, not a build one: the repository kernel holds no
# git library and drives the ordinary plumbing through the command.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git && rm -rf /var/lib/apt/lists/*
# The group is pinned because the chart hands the seat over by fsGroup and the
# two numbers have to agree. Left to useradd --system the uid lands at 10001
# and the gid at whatever the system range has free.
RUN groupadd --gid 10001 codehull \
    && useradd --uid 10001 --gid 10001 --home /codehull --shell /usr/sbin/nologin codehull
WORKDIR /codehull
COPY deploy/codehull-api /usr/local/bin/codehull-api
COPY deploy/codehull.toml codehull.toml
RUN chmod 0755 /usr/local/bin/codehull-api && mkdir -p .local && chown -R codehull:codehull /codehull
USER codehull
EXPOSE 3400
# The subcommand and its root arrive as args: bootstrap or serve.
ENTRYPOINT ["codehull-api"]
CMD ["serve", "/codehull"]
