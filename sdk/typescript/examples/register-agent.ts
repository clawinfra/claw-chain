/**
 * Example: Register an EvoClaw agent on ClawChain testnet
 *
 * Run with:
 *   npx ts-node --esm register-agent.ts
 *   # or after building:
 *   node ../dist/examples/register-agent.js
 *
 * Environment variables (optional):
 *   SIGNER_MNEMONIC — 12-word mnemonic for the signing account
 *                     Default: "//Alice" dev account (testnet only!)
 *   CLAWCHAIN_WS    — WebSocket endpoint (default: wss://testnet.clawchain.win)
 */

import { ClawChainClient, AgentRegistry } from "../src/index.js";
import { Keyring } from "@polkadot/keyring";
import { cryptoWaitReady } from "@polkadot/util-crypto";

async function main(): Promise<void> {
  // ── 1. Crypto initialisation (required for sr25519) ──────────────────────
  await cryptoWaitReady();

  // ── 2. Connect to ClawChain ───────────────────────────────────────────────
  const wsUrl = process.env["CLAWCHAIN_WS"] ?? "wss://testnet.clawchain.win";
  console.log(`Connecting to ${wsUrl} ...`);

  const client = new ClawChainClient(wsUrl);
  await client.connect();

  const { chainName, specVersion } = await client.getChainInfo();
  console.log(`✓ Connected — ${chainName} (spec v${specVersion})`);

  // ── 3. Load signer ────────────────────────────────────────────────────────
  const keyring = new Keyring({ type: "sr25519" });
  const mnemonic = process.env["SIGNER_MNEMONIC"] ?? "//Alice";
  const signer = keyring.addFromUri(mnemonic);

  console.log(`Signer address: ${signer.address}`);

  // Show current balance
  const balance = await client.getBalance(signer.address);
  console.log(`Balance: ${ClawChainClient.formatBalance(balance)}`);

  if (balance === 0n) {
    console.warn(
      "⚠️  Balance is 0. You need testnet CLAW to pay transaction fees."
    );
    console.warn(
      "   On the testnet dev chain, //Alice has pre-funded balance."
    );
  }

  // ── 4. Define agent identity ──────────────────────────────────────────────
  // In production, generate a stable DID for your agent.
  const agentDid = `did:claw:agent:evoclaw-${Date.now()}`;

  const agentMetadata = {
    name: "EvoClaw Demo Agent",
    type: "task-executor",
    capabilities: ["text-summarisation", "translation", "code-review"],
    version: "1.0.0",
    endpoint: "https://agent.example.com/api",
    createdAt: new Date().toISOString(),
  };

  console.log(`\nRegistering agent...`);
  console.log(`  DID:  ${agentDid}`);
  console.log(`  Name: ${agentMetadata.name}`);

  // ── 5. Register on-chain ──────────────────────────────────────────────────
  const registry = new AgentRegistry(client);

  try {
    const blockHash = await registry.registerAgent(
      signer,
      agentDid,
      agentMetadata
    );
    console.log(`\n✅ Agent registered!`);
    console.log(`   Block hash: ${blockHash}`);
  } catch (err) {
    console.error(`\n❌ Registration failed:`, err);
    process.exit(1);
  }

  // ── 6. Verify on-chain ────────────────────────────────────────────────────
  console.log(`\nVerifying registration...`);
  const allAgents = await registry.listAgents();
  const myAgent = allAgents.find((a) => a.did === agentDid);

  if (myAgent) {
    console.log(`\n✅ Agent confirmed on-chain:`);
    console.log(`   Agent ID:   ${myAgent.agentId}`);
    console.log(`   Owner:      ${myAgent.owner}`);
    console.log(`   DID:        ${myAgent.did}`);
    console.log(`   Reputation: ${myAgent.reputation} / 10,000`);
    console.log(`   Status:     ${myAgent.status}`);
    console.log(`   Block:      ${myAgent.registeredAt}`);
  } else {
    console.warn("⚠️  Agent not found in on-chain list (may need a moment to index).");
  }

  // ── 7. Disconnect ─────────────────────────────────────────────────────────
  await client.disconnect();
  console.log(`\n👋 Disconnected. Done!`);
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
