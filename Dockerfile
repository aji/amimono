FROM rust:1.91-slim-bullseye AS build
WORKDIR /usr/src/example-adder
COPY . .
RUN cargo install --path example-adder

FROM debian:bullseye-slim
COPY --from=build /usr/local/cargo/bin/example-adder /usr/local/bin/example-adder
CMD ["example-adder"]
