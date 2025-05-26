###############################################################################
# BEGIN build-stage
# Compile the binary
FROM --platform=$BUILDPLATFORM docker.io/library/golang:1.24.3 AS build-stage

ARG BUILDPLATFORM
ARG TARGETARCH

WORKDIR /app

COPY . ./

RUN GOOS=linux GOARCH="${TARGETARCH}" hack/build.sh

#
# END build-stage
###############################################################################

###############################################################################
# BEGIN final-stage
# Create final docker image
FROM scratch AS final-stage

COPY --from=build-stage /app/bin/cerberus-mergeguard /cerberus-mergeguard

USER 1001

ENTRYPOINT ["/cerberus-mergeguard"]

#
# END final-stage
###############################################################################
