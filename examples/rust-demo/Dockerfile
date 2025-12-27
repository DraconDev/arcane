FROM rust:1.80-alpine as builder
WORKDIR /app
COPY . .
RUN apk add --no-cache musl-dev
RUN cargo build --release --bin rust-demo

FROM alpine:3.18
WORKDIR /app
COPY --from=builder /app/target/release/rust-demo .
CMD ["./rust-demo"]
