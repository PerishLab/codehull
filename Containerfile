FROM docker.io/library/debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git && rm -rf /var/lib/apt/lists/*
RUN groupadd --gid 10001 codehull \
    && useradd --uid 10001 --gid 10001 --home /codehull --shell /usr/sbin/nologin codehull
WORKDIR /codehull
COPY codehull-api /usr/local/bin/codehull-api
RUN chmod 0755 /usr/local/bin/codehull-api && mkdir -p .local && chown -R codehull:codehull /codehull
USER codehull
EXPOSE 3400
ENTRYPOINT ["codehull-api"]
CMD ["serve", "/codehull"]
