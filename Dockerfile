FROM rust:1.72 AS builder

COPY . /checker-compile
WORKDIR /checker-compile

RUN cargo build --release

FROM debian:bullseye-slim

RUN rm -rf /var/lib/apt/lists/*

RUN mkdir -p /lira-checker
COPY --from=builder /checker-compile/target/release/lira-checker /lira-checker/checker
COPY ./supported_currencies.json /lira-checker/supported_currencies.json

RUN groupadd --system checker && useradd --system checker --gid checker && chown -R checker:checker /lira-checker
USER checker:checker

WORKDIR /lira-checker
CMD ["./checker"]