FROM rust:latest AS builder

WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release

FROM debian:stable-slim

WORKDIR /app
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  # Clean up
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/myapp/*/release/brain /usr/local/bin/
COPY --from=builder /usr/src/myapp/*/release/kraken /usr/local/bin/
ENV TZ=Europe/Berlin

CMD ["ls"]
