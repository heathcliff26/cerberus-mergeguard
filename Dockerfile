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

ARG CI_COMMIT_SHA=unknown

RUN cargo build --release

#
# END build-stage
###############################################################################

###############################################################################
# BEGIN final-stage
# Create final docker image
FROM docker.io/library/alpine:3.23.2@sha256:865b95f46d98cf867a156fe4a135ad3fe50d2056aa3f25ed31662dff6da4eb62 AS final-stage

COPY --from=build-stage /app/target/release/cerberus-mergeguard /usr/local/bin/cerberus-mergeguard

WORKDIR /config

USER 1001

ENTRYPOINT ["cerberus-mergeguard"]

CMD ["server"]

#
# END final-stage
###############################################################################
