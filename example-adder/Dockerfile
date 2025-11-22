FROM rust:1-slim-trixie AS build
WORKDIR /app
COPY . .
RUN cargo build -p example-adder --release

FROM debian:trixie-slim
WORKDIR /app
COPY --from=build /app/target/release/example-adder /app/example-adder
ENTRYPOINT ["/app/example-adder"]