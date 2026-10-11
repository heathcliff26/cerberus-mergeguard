###############################################################################
# BEGIN build-stage
# Compile the binary
FROM docker.io/library/rust:1.98.1-alpine AS build-stage

WORKDIR /app

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Needed as we include it for docs.
RUN touch README.md

ARG CI_COMMIT_SHA=unknown

RUN cargo build --release

#
# END build-stage
###############################################################################

###############################################################################
# BEGIN final-stage
# Create final docker image
FROM docker.io/library/alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6 AS final-stage

COPY --from=build-stage /app/target/release/cerberus-mergeguard /usr/local/bin/cerberus-mergeguard

WORKDIR /config

USER nobody:nobody

ENTRYPOINT ["cerberus-mergeguard"]

CMD ["server"]

#
# END final-stage
###############################################################################
