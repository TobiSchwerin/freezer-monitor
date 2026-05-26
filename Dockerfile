# Stage 1: React-Frontend bauen
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Rust-Backend bauen
FROM rust:1.85-slim AS backend-builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Stage 3: Schlankes Endimage (nur das Nötigste)
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /app/target/release/freezer-monitor ./
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist
RUN mkdir -p firmware
EXPOSE 3000
CMD ["./freezer-monitor"]