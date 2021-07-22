FROM rust:1.45.2 as builder
WORKDIR /usr/src/
RUN git clone https://github.com/ergoplatform/oracle-core.git
WORKDIR /usr/src/oracle-core
RUN cargo build --release
RUN cargo install --path .

FROM rust:1.45.2-slim
COPY --from=builder /usr/local/cargo/bin/oracle-core /usr/local/bin/oracle-core
CMD ["oracle-core"]
