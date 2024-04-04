FROM rust:1.77-bullseye as builder

WORKDIR /scraper
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo install --path .

FROM debian:bullseye
RUN apt-get update && apt install libssl-dev libudev-dev
WORKDIR /scraper
COPY --from=builder /usr/local/cargo/bin/sdr-scraper /usr/local/bin/sdr-scraper

EXPOSE 3000

CMD ["sdr-scraper"]
