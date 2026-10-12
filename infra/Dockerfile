FROM node:22.22.0-bookworm-slim AS web
WORKDIR /src
RUN npm install --global pnpm@10.29.3
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps ./apps
COPY packages ./packages
RUN pnpm install --frozen-lockfile
RUN pnpm build:web

FROM rust:1.99.0-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY migrations ./migrations
COPY apps/desktop ./apps/desktop
RUN cargo build --locked --release -p faktura-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl util-linux && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home faktura && mkdir -p /data/files && chown faktura:faktura /data/files
COPY --from=rust /src/target/release/faktura-server /usr/local/bin/faktura-server
COPY --from=web /src/apps/web/dist /app/web
ENV API_ADDRESS=0.0.0.0:3001 STATIC_DIR=/app/web APP_ENV=production
COPY infra/entrypoint.sh /usr/local/bin/faktura-entrypoint
RUN chmod 0755 /usr/local/bin/faktura-entrypoint
EXPOSE 3001
ENTRYPOINT ["faktura-entrypoint"]
