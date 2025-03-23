FROM rust:latest AS builder

WORKDIR /root/build

COPY . .

RUN cargo build --release

# Runtime
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 libpq5 && apt-get clean

COPY --from=builder /root/build/target/release/wss-service ./

ENTRYPOINT ["/app/wss-service"]