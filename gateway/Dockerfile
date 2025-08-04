FROM rust:latest AS builder
WORKDIR /app
COPY . .
ENV RUSTFLAGS="-C target-cpu=skylake"
RUN cargo build --release
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/gateway .
ENTRYPOINT [ "./gateway" ]
