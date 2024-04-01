FROM rust:1.67

WORKDIR /scraper
COPY . .

RUN cargo install --path .

CMD ["sdr-scraper"]
