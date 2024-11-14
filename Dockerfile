FROM rust:1.81.0 AS builder

WORKDIR /app 

RUN apt update && apt install lld clang -y

COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release

# FROM rust:1.80.1 AS runtime
FROM debian:latest AS runtime

WORKDIR /app 

COPY --from=builder /app/target/release/flip_pine flip_pine
RUN apt-get update -y \
  && apt-get install -y  openssl ca-certificates \
  # && apt install openssl ca-certificates libssl-dev \
  # Clean up
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*
COPY configurations configurations
ENV APP_ENV production

ENTRYPOINT ["./flip_pine"]
