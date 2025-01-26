
# STAGE ONE: COMPILE-TIME
# From base image and give it a tag
FROM rust:1.81.0 AS builder

# Set working directory, or it's set to default /root
WORKDIR /app 

# install a compiler and a linker
RUN apt update && apt install lld clang -y

# Copy source code
COPY . .
# Setup environment variables
ENV SQLX_OFFLINE true
# Use build tools to build target
RUN cargo build --release

# STAGE TWO: RUNTIME
FROM debian:latest AS runtime

WORKDIR /app 

# Copy from souce to target
COPY --from=builder /app/target/release/flip_pine flip_pine
# INSTALL RUNTIME DEPENDENCIES
RUN apt-get update -y \
  && apt-get install -y  openssl ca-certificates \
  # Clean up
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*
# Copy configurations
COPY configurations configurations
# Set runtime environment variables
ENV APP_ENV production

# Set which command to run when start the container
ENTRYPOINT ["./flip_pine"]
