FROM rust:1.45.2 as builder
WORKDIR /usr/src/
RUN git clone https://github.com/ergoplatform/oracle-core.git
WORKDIR /usr/src/oracle-core/connectors/erg-usd-connector
RUN cargo build --release
RUN cargo install --path .

FROM rust:1.45.2-slim
COPY --from=builder /usr/local/cargo/bin/erg-usd-connector /usr/local/bin/erg-usd-connector
CMD ["erg-usd-connector"]
