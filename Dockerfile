FROM rust:alpine AS build-stage

RUN apk update
RUN apk add cmake make musl-dev g++ perl ca-certificates

WORKDIR /build
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# Build image from scratch
FROM scratch
LABEL org.opencontainers.image.source="https://github.com/pcvolkmer/psn-to-pid"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"
LABEL org.opencontainers.image.description="Request original (PID) for given gPAS PSN"

COPY --from=build-stage /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build-stage /build/target/release/mv64e-psn-to-pid .

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs

USER 65532

EXPOSE 3000
CMD ["./psn-to-pid"]