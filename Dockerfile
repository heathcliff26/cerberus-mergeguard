###############################################################################
# BEGIN build-stage
# Compile the binary
FROM docker.io/library/rust:1.99.1-alpine AS build-stage

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
FROM docker.io/library/alpine:3.24.4@sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf AS final-stage

COPY --from=build-stage /app/target/release/cerberus-mergeguard /usr/local/bin/cerberus-mergeguard

WORKDIR /config

USER nobody:nobody

ENTRYPOINT ["cerberus-mergeguard"]

CMD ["server"]

#
# END final-stage
###############################################################################
