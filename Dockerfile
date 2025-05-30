###############################################################################
# BEGIN build-stage
# Compile the binary
FROM docker.io/library/rust:1.87.0 AS build-stage

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Needed as we include it for docs.
RUN touch README.md

RUN cargo build --release

#
# END build-stage
###############################################################################

###############################################################################
# BEGIN final-stage
# Create final docker image
FROM docker.io/library/debian:12.11-slim AS final-stage

COPY --from=build-stage /app/target/release/cerberus-mergeguard /cerberus-mergeguard

WORKDIR /config
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

USER 1001

ENTRYPOINT ["/cerberus-mergeguard"]

CMD ["server"]

#
# END final-stage
###############################################################################
