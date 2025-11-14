FROM rust:1.91-slim-bullseye AS build
WORKDIR /usr/src/example_adder
COPY . .
RUN cargo install --path example_adder

FROM debian:bullseye-slim
COPY --from=build /usr/local/cargo/bin/example_adder /usr/local/bin/example_adder
CMD ["example_adder"]