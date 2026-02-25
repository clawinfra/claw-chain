# ClawChain Testnet Faucet

A production-grade faucet service for the ClawChain testnet. Provides free CLAW tokens to developers building on the network.

**Live:** https://faucet.clawchain.win

---

## Features

- **100 CLAW** per address per 24 hours (standard)
- **1000 CLAW** with GitHub OAuth (10× boost)
- **IP rate limiting:** max 10 requests per hour per IP
- **SQLite** backend — zero infrastructure dependencies
- **Single-page frontend** — dark theme, countdown timers, no build step
- **SS58 address validation** via `@polkadot/util-crypto`
- **Direct Substrate transfer** via `@polkadot/api`

---

## Architecture

```
services/faucet/
├── src/
│   ├── index.ts              # Entry point — boots app, connects chain
│   ├── server.ts             # Express app factory
│   ├── config.ts             # .env loader + typed config
│   ├── db.ts                 # SQLite schema + query helpers
│   ├── chain.ts              # Polkadot API connect + sr25519 transfer
│   ├── routes/
│   │   ├── faucet.ts         # POST /faucet
│   │   ├── status.ts         # GET /status
│   │   └── auth.ts           # GitHub OAuth (/auth/github, /auth/me, /auth/logout)
│   └── middleware/
│       └── rateLimit.ts      # IP-based rate limiting
├── public/
│   └── index.html            # SPA frontend (inline CSS + JS, no build)
└── tests/
    ├── db.test.ts             # SQLite helpers unit tests
    ├── faucet.test.ts         # API route integration tests (mocked chain)
    └── rateLimit.test.ts      # Middleware unit tests
```

### API

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/faucet` | Request CLAW tokens. Body: `{ address: string }` |
| `GET` | `/status` | Faucet balance + stats |
| `GET` | `/auth/github` | GitHub OAuth redirect |
| `GET` | `/auth/github/callback` | OAuth callback |
| `GET` | `/auth/me` | `{ authenticated, username? }` |
| `GET` | `/auth/logout` | Destroy session |

#### POST /faucet

**Request:**
```json
{ "address": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
```

**Success (200):**
```json
{
  "tx_hash": "0xabcdef...",
  "amount": "100 CLAW",
  "next_drip_at": "2024-01-02T00:00:00.000Z"
}
```

**Rate limited (429):**
```json
{
  "error": "Address rate limited",
  "next_drip_at": "2024-01-02T00:00:00.000Z"
}
```

---

## Setup

### Prerequisites

- Node.js 18+
- Access to a ClawChain (Substrate) node
- A funded faucet account

### Local Development

```bash
# 1. Install dependencies
cd services/faucet
npm install

# 2. Configure environment
cp .env.example .env
# Edit .env with your values:
#   RPC_URL=ws://your-node:9944
#   FAUCET_SEED=//Alice  (or a real mnemonic for testnet)

# 3. Start dev server
npm run dev
# → http://localhost:3000

# 4. Run tests
npm test
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RPC_URL` | ✅ | — | WebSocket URL of Substrate node |
| `FAUCET_SEED` | ✅ | — | Sr25519 seed/URI for faucet account |
| `GITHUB_CLIENT_ID` | — | `""` | GitHub OAuth App client ID |
| `GITHUB_CLIENT_SECRET` | — | `""` | GitHub OAuth App secret |
| `SESSION_SECRET` | — | `change-me` | Express session signing key |
| `PORT` | — | `3000` | HTTP listen port |
| `DB_PATH` | — | `./faucet.db` | SQLite file path |

### GitHub OAuth (optional — enables 1000 CLAW boost)

1. Create a GitHub OAuth App at https://github.com/settings/developers
2. Set **Homepage URL:** `https://faucet.clawchain.win`
3. Set **Callback URL:** `https://faucet.clawchain.win/auth/github/callback`
4. Copy the Client ID and Secret into `.env`

---

## Deployment to faucet.clawchain.win

### Docker

```bash
# Build image
docker build -t clawchain-faucet .

# Run
docker run -d \
  --name faucet \
  --restart unless-stopped \
  -p 3000:3000 \
  -v /srv/faucet-data:/data \
  -e RPC_URL=ws://your-clawchain-node:9944 \
  -e FAUCET_SEED="your twelve word mnemonic phrase here" \
  -e GITHUB_CLIENT_ID=your_client_id \
  -e GITHUB_CLIENT_SECRET=your_client_secret \
  -e SESSION_SECRET=$(openssl rand -hex 32) \
  clawchain-faucet
```

### Nginx Reverse Proxy

```nginx
server {
    listen 443 ssl;
    server_name faucet.clawchain.win;

    ssl_certificate /etc/letsencrypt/live/faucet.clawchain.win/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/faucet.clawchain.win/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Docker Compose

```yaml
version: '3.8'

services:
  faucet:
    build: .
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      RPC_URL: ws://clawchain-node:9944
      FAUCET_SEED: ${FAUCET_SEED}
      GITHUB_CLIENT_ID: ${GITHUB_CLIENT_ID}
      GITHUB_CLIENT_SECRET: ${GITHUB_CLIENT_SECRET}
      SESSION_SECRET: ${SESSION_SECRET}
    volumes:
      - faucet-data:/data

volumes:
  faucet-data:
```

---

## Integration Testing (manual)

Start a local Substrate node (e.g. `substrate --dev`) and run:

```bash
# Start faucet
RPC_URL=ws://localhost:9944 FAUCET_SEED=//Alice npm run dev

# Request CLAW via curl
curl -X POST http://localhost:3000/faucet \
  -H 'Content-Type: application/json' \
  -d '{"address":"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"}'

# Check status
curl http://localhost:3000/status
```

Verify the balance changed on-chain using Polkadot.js Apps or `subxt`.

---

## Rate Limits

| Limit | Value |
|-------|-------|
| CLAW per address per 24h | 100 |
| CLAW per address per 24h (GitHub OAuth) | 1000 |
| Requests per hour per IP | 10 |

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Runtime | Node.js 18+ (ESM) |
| Language | TypeScript 5 (strict) |
| HTTP | Express 4 |
| Database | SQLite via better-sqlite3 |
| Chain | @polkadot/api (sr25519) |
| Auth | passport-github2 + express-session |
| Tests | Vitest + supertest |
| Container | Docker (node:18-slim, multi-stage) |

---

## License

MIT — part of the [ClawChain](https://github.com/clawinfra/claw-chain) project.
