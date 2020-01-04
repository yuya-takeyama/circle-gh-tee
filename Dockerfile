FROM rust:1.40-slim

COPY . /app
WORKDIR /app
RUN apt-get update && \
  apt-get install -y libssl-dev pkg-config && \
  cargo build --release && \
  cp /app/target/release/circle-gh-tee /usr/local/bin && \
  rm -rf /app
