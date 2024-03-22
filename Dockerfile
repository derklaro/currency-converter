FROM rust:1.77 AS builder

COPY . /currency-compile
WORKDIR /currency-compile

RUN cargo build --release

FROM debian:bookworm-slim

RUN rm -rf /var/lib/apt/lists/*

RUN mkdir -p /currency-converter
COPY --from=builder /currency-compile/target/release/currency-converter /currency-converter/currency
COPY ./supported_currencies.json /currency-converter/supported_currencies.json

RUN groupadd --system currency && useradd --system currency --gid currency && chown -R currency:currency /currency-converter
USER currency:currency

WORKDIR /currency-converter
CMD ["./currency"]
