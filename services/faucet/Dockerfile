# ─── Stage 1: Build ─────────────────────────────────────────────────────────
FROM node:18-slim AS builder

WORKDIR /app

# Install build tools needed for better-sqlite3 native module
RUN apt-get update && apt-get install -y \
    python3 \
    make \
    g++ \
    && rm -rf /var/lib/apt/lists/*

COPY package.json ./
RUN npm install

COPY tsconfig.json ./
COPY src/ ./src/

RUN npm run build

# ─── Stage 2: Runtime ────────────────────────────────────────────────────────
FROM node:18-slim AS runtime

WORKDIR /app

# Install runtime dependencies for better-sqlite3
RUN apt-get update && apt-get install -y \
    python3 \
    make \
    g++ \
    && rm -rf /var/lib/apt/lists/*

COPY package.json ./

# Install production deps only (with native rebuild)
RUN npm install --omit=dev

# Copy built output + public assets
COPY --from=builder /app/dist ./dist
COPY public/ ./public/

# Create data directory for SQLite
RUN mkdir -p /data && chown node:node /data

USER node

EXPOSE 3000

ENV NODE_ENV=production
ENV DB_PATH=/data/faucet.db

CMD ["node", "dist/index.js"]
