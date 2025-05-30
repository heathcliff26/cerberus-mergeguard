###############################################################################
# BEGIN build-stage
# Compile the binary
FROM docker.io/library/rust:alpine3.21 AS build-stage

WORKDIR /app

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static

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
FROM docker.io/library/alpine:3.21 AS final-stage

COPY --from=build-stage /app/target/release/cerberus-mergeguard /usr/local/bin/cerberus-mergeguard

WORKDIR /config

USER 1001

ENTRYPOINT ["cerberus-mergeguard"]

CMD ["server"]

#
# END final-stage
###############################################################################
