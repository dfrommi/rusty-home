FROM rust:latest AS builder

WORKDIR /usr/src/myapp

## Dependency caching
COPY Cargo.toml Cargo.lock ./
COPY app/Cargo.toml ./app/
COPY lib/macro/Cargo.toml ./lib/macro/
COPY lib/infrastructure/Cargo.toml ./lib/infrastructure/

RUN mkdir -p app/src kraken/src lib/macro/src lib/infrastructure/src \
  && echo "fn main() {}" > app/src/main.rs \
  && echo "#[proc_macro] pub fn dummy(_: proc_macro::TokenStream) -> proc_macro::TokenStream {proc_macro::TokenStream::new()}" > lib/macro/src/lib.rs \
  && echo "pub fn dummy() {}" > lib/infrastructure/src/lib.rs

RUN cargo fetch
RUN cargo build --release
RUN rm -rf app/src lib/macro/src lib/infrastructure/src
## end of dependency caching

COPY . .

#bypass cargo's caching and force rebuild
RUN touch -a -m app/src/main.rs lib/macro/src/lib.rs lib/infrastructure/src/lib.rs

RUN cargo build --release

FROM debian:stable-slim

WORKDIR /app
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  # Clean up
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

EXPOSE 8080

COPY --from=builder /usr/src/myapp/*/release/app /usr/local/bin/
ENV TZ=Europe/Berlin

CMD ["app"]
