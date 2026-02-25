# ClawChain Explorer

A real-time block explorer for the [ClawChain](https://clawchain.win) agent-native L1 blockchain.

Built with Next.js 14 App Router, TypeScript (strict mode), Tailwind CSS, and [@polkadot/api](https://polkadot.js.org/docs/api/).

## Features

- 🔴 **Live block feed** — real-time subscribeNewHeads with rolling 50-block window
- 🔍 **Block detail** — full extrinsic list with success/failure status
- 📄 **Transaction detail** — args, events, fee, tip decoded on-chain
- 🤖 **Agent profiles** — DID, reputation score + history, gas quota
- ⚡ **Auto-reconnect** — exponential backoff (1s → 2s → 4s → max 30s)
- 🌑 **Dark theme** — ClawChain brand colors (#0a0a0a bg, #00D4FF accent)
- 📱 **Responsive** — desktop-first, works on mobile

## Architecture

```
Browser → @polkadot/api (WebSocket) → ClawChain Node
```

No backend. All data fetched client-side directly from the chain via WebSocket RPC.

**Pallets supported:**
- `pallet-agent-registry` — agent DID + registry data
- `pallet-reputation` — on-chain reputation scores + history
- `pallet-gas-quota` — per-agent gas quota tracking

All pallet queries are wrapped in try/catch. If a pallet isn't available in the runtime version, the UI shows an "Unavailable" badge instead of crashing.

## Getting Started

```bash
# Install dependencies
npm install

# Copy env
cp .env.example .env.local
# Edit NEXT_PUBLIC_WS_URL to point to your node

# Run dev server
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `NEXT_PUBLIC_WS_URL` | `wss://testnet.clawchain.win` | WebSocket RPC URL |

## Running Tests

```bash
npm test                    # run all tests once
npm run test:watch          # watch mode
npm run test:coverage       # with coverage report
```

Coverage targets: ≥90% for `lib/` and `hooks/`, ≥80% for `components/`.

## Building for Production

```bash
npm run build
npm start
```

## Docker

```bash
# Build
docker build -t clawchain-explorer \
  --build-arg NEXT_PUBLIC_WS_URL=wss://testnet.clawchain.win \
  .

# Run
docker run -p 3000:3000 clawchain-explorer
```

## Deployment

Target: `explorer.clawchain.win`

The Docker image uses Next.js standalone output for minimal footprint. Mount `.env.local` or pass `NEXT_PUBLIC_WS_URL` as a build arg.

## File Structure

```
src/
├── app/               # Next.js 14 App Router pages
│   ├── blocks/        # Block list + detail
│   ├── tx/[hash]/     # Transaction detail
│   └── agents/[address]/  # Agent profile
├── components/        # Presentational components
├── hooks/             # Data-fetching hooks
├── lib/               # Utilities and types
└── providers/         # React context (ApiProvider)
```

## License

Part of the [ClawChain](https://github.com/clawinfra/claw-chain) project. See root LICENSE.
