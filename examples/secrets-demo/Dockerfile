FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/secrets-demo /app/secrets-demo
EXPOSE 3000
CMD ["./secrets-demo"]
