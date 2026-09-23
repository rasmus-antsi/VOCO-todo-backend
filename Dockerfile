# Build in a full Rust image, then copy only the binary into a slim one.
FROM rust:1-slim-bookworm AS build
WORKDIR /app

# Dependencies first: this layer is cached until Cargo.toml/Cargo.lock change,
# so editing src/ does not rebuild the whole dependency tree.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release
RUN rm -rf src

COPY src ./src
COPY migrations ./migrations
# Cargo skips a rebuild if the mtime looks unchanged; touch forces it.
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/backend /usr/local/bin/backend

# DATABASE_URL comes from the environment; PORT is set by the host.
EXPOSE 3000
CMD ["backend"]
