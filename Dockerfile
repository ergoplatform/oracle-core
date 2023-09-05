# Valid CCARCH options: aarch64 or the default x86_64 
ARG CCARCH="x86_64"

FROM --platform=$BUILDPLATFORM messense/rust-musl-cross:${CCARCH}-musl AS builder
WORKDIR /home/rust/src
COPY . .

RUN rm rust-toolchain

RUN cargo build --release

RUN apt-get update && apt-get --only-upgrade install -y ca-certificates
RUN update-ca-certificates

RUN adduser \
    --disabled-password \
    --gecos "oracle-core" \
    --home "/data" \
    --shell "/bin/sh" \
    --no-create-home \
    --uid "9010" \
    oracle-core

FROM --platform=$TARGETPLATFORM busybox:stable-musl AS final
ARG CCARCH="x86_64"
COPY --from=builder /home/rust/src/target/${CCARCH}-unknown-linux-musl/release/oracle-core /usr/local/bin/oracle-core
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /etc/passwd /etc/passwd

EXPOSE 9010 9011

USER oracle-core

CMD ["oracle-core", "--oracle-config-file", "/data/oracle_config.yaml", "--pool-config-file", "/data/pool_config.yaml", "-d", "/data", "run", "--enable-rest-api"]
