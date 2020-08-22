FROM rust:1.45.2 as builder
WORKDIR /usr/src/
RUN git clone https://github.com/ergoplatform/oracle-core.git
WORKDIR /usr/src/oracle-core/connectors/ada-usd-connector
RUN cargo build --release
RUN cargo install --path .

FROM rust:1.45.2-slim
COPY --from=builder /usr/local/cargo/bin/ada-usd-connector /usr/local/bin/ada-usd-connector
CMD ["ada-usd-connector"]
