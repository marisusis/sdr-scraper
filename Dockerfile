FROM rust:1.76

WORKDIR /scraper
COPY . .

RUN cargo install --path .

CMD ["sdr-scraper"]
