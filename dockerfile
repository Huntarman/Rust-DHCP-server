# Build Stage
FROM rust:1.80.1 AS builder
WORKDIR /usr/src/rust-dhcp-server

COPY server/Cargo.toml server/Cargo.lock ./
COPY server/src ./src
COPY server-config.json ./server-config.json

RUN cargo build --release

FROM debian:bookworm-slim

ENV RUST_LOG=info

RUN apt-get update && apt-get install -y libssl-dev iproute2 iputils-ping postgresql-client && rm -rf /var/lib/apt/lists/*

COPY wait-for-db.sh /usr/local/bin/wait-for-db.sh
RUN chmod +x /usr/local/bin/wait-for-db.sh

COPY --from=builder /usr/src/rust-dhcp-server/target/release/server /usr/local/bin/server
COPY --from=builder  /usr/src/rust-dhcp-server/server-config.json /app/server-config.json

ENTRYPOINT ["/usr/local/bin/wait-for-db.sh", "/usr/local/bin/server"]

EXPOSE 67/udp
