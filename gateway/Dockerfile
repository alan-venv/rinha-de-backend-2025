FROM rust:latest AS builder
WORKDIR /app

##
COPY Cargo.toml ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -r src
##

COPY . .
RUN cargo build --release
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/gateway .
EXPOSE 8080
ENTRYPOINT [ "./gateway" ]
