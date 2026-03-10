# ClawChain Testnet Faucet — Docker Image
#
# Multi-stage build:
#   Stage 1 (builder):  Node.js + TypeScript → compiles faucet
#   Stage 2 (runtime):  Node.js slim → minimal production image
#
# Build:
#   docker build -t clawchain-faucet:latest .
#
# Run:
#   docker run --rm -it \
#     -p 3001:3001 \
#     -e RPC_URL=ws://clawchain-validator:9944 \
#     -e FAUCET_SEED="//Alice" \
#     clawchain-faucet:latest

# ── Stage 1: Builder ──────────────────────────────────────────────────────────
FROM node:20-slim AS builder

WORKDIR /build

COPY package.json package-lock.json* ./
RUN npm ci --ignore-scripts

COPY tsconfig.json ./
COPY src/ ./src/
RUN npm run build

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM node:20-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends tini && \
    rm -rf /var/lib/apt/lists/*

# Non-root service account
RUN useradd --system --uid 1001 --gid 0 \
    --no-create-home --shell /usr/sbin/nologin \
    faucet

WORKDIR /app

# Copy compiled output and production deps
COPY --from=builder /build/dist ./dist
COPY package.json ./
RUN npm ci --omit=dev --ignore-scripts

USER faucet

EXPOSE 3001

HEALTHCHECK \
    --interval=30s \
    --timeout=10s \
    --start-period=30s \
    --retries=3 \
    CMD node -e "require('http').get('http://localhost:3001/status', r => process.exit(r.statusCode === 200 ? 0 : 1)).on('error', () => process.exit(1))"

ENV PORT=3001 \
    DRIP_AMOUNT_CLAW=1000 \
    COOLDOWN_HOURS=24 \
    IP_RATE_LIMIT=10 \
    LOG_LEVEL=info \
    LOG_PRETTY=false

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["node", "dist/index.js"]
