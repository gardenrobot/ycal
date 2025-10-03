FROM rust:1.86 AS builder
WORKDIR /usr/src/ycal
COPY src/ src/
COPY tests/ tests/
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN cargo install --path .

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/ycal .
COPY run_range.sh .
CMD ["/app/run_range.sh"]
