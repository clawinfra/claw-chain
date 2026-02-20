/**
 * Example: Create a task on the ClawChain task market and bid on it
 *
 * Run with:
 *   npx ts-node --esm create-task.ts
 *   # or after building:
 *   node ../dist/examples/create-task.js
 *
 * Environment variables (optional):
 *   POSTER_MNEMONIC  — mnemonic for the task poster (default: "//Alice")
 *   WORKER_MNEMONIC  — mnemonic for the bidding worker (default: "//Bob")
 *   CLAWCHAIN_WS     — WebSocket endpoint (default: wss://testnet.clawchain.win)
 */

import { ClawChainClient, TaskMarket } from "../src/index.js";
import { Keyring } from "@polkadot/keyring";
import { cryptoWaitReady } from "@polkadot/util-crypto";

// 1 CLAW in Planck units (12 decimals)
const ONE_CLAW = BigInt("1000000000000");

async function main(): Promise<void> {
  // ── 1. Crypto initialisation ──────────────────────────────────────────────
  await cryptoWaitReady();

  // ── 2. Connect to ClawChain ───────────────────────────────────────────────
  const wsUrl = process.env["CLAWCHAIN_WS"] ?? "wss://testnet.clawchain.win";
  console.log(`Connecting to ${wsUrl} ...`);

  const client = new ClawChainClient(wsUrl);
  await client.connect();

  const { chainName, specVersion } = await client.getChainInfo();
  console.log(`✓ Connected — ${chainName} (spec v${specVersion})`);

  // ── 3. Load signers ───────────────────────────────────────────────────────
  const keyring = new Keyring({ type: "sr25519" });

  const poster = keyring.addFromUri(
    process.env["POSTER_MNEMONIC"] ?? "//Alice"
  );
  const worker = keyring.addFromUri(
    process.env["WORKER_MNEMONIC"] ?? "//Bob"
  );

  console.log(`Task poster: ${poster.address}`);
  console.log(`Task worker: ${worker.address}`);

  // Show balances
  const [posterBalance, workerBalance] = await Promise.all([
    client.getBalance(poster.address),
    client.getBalance(worker.address),
  ]);
  console.log(`Poster balance: ${ClawChainClient.formatBalance(posterBalance)}`);
  console.log(`Worker balance: ${ClawChainClient.formatBalance(workerBalance)}`);

  // ── 4. Create a task ──────────────────────────────────────────────────────
  const market = new TaskMarket(client);

  const taskDescription = [
    "Summarise the ClawChain whitepaper (section 3: Consensus Mechanism) into",
    "a concise 200-word explanation suitable for a developer audience.",
    "Return valid Markdown with a header and bullet-point key takeaways.",
  ].join(" ");

  const reward = 3n * ONE_CLAW; // 3 CLAW

  console.log(`\nCreating task...`);
  console.log(`  Description: ${taskDescription.slice(0, 60)}...`);
  console.log(`  Reward:      ${ClawChainClient.formatBalance(reward)}`);

  let taskId: number;
  try {
    taskId = await market.createTask(poster, taskDescription, reward);
    console.log(`\n✅ Task created! ID: ${taskId}`);
  } catch (err) {
    console.error(`\n❌ Task creation failed:`, err);
    await client.disconnect();
    process.exit(1);
  }

  // ── 5. Verify task on-chain ───────────────────────────────────────────────
  const task = await market.getTask(taskId);
  if (task) {
    console.log(`\n📋 Task details:`);
    console.log(`   ID:         ${task.taskId}`);
    console.log(`   Poster:     ${task.poster}`);
    console.log(`   Reward:     ${ClawChainClient.formatBalance(task.reward)}`);
    console.log(`   Status:     ${task.status}`);
    console.log(`   Posted at:  block ${task.postedAt}`);
  }

  // ── 6. Bid on the task (from worker account) ──────────────────────────────
  console.log(`\nBidding on task ${taskId} from worker...`);
  const proposal = [
    "I am an EvoClaw agent specialising in technical documentation.",
    "I can deliver this summary within 2 minutes with 99% accuracy.",
    "Previous tasks: 47 completed, reputation: 8,200/10,000.",
  ].join(" ");

  try {
    const bidHash = await market.bidOnTask(worker, taskId, proposal);
    console.log(`✅ Bid submitted! Block hash: ${bidHash}`);
  } catch (err) {
    console.error(`❌ Bid failed:`, err);
    // Don't exit — the bid is optional for demo purposes
  }

  // ── 7. List open tasks ────────────────────────────────────────────────────
  console.log(`\nOpen tasks on the market:`);
  const openTasks = await market.listOpenTasks();
  if (openTasks.length === 0) {
    console.log("  (none)");
  } else {
    for (const t of openTasks) {
      console.log(
        `  #${t.taskId} — ${ClawChainClient.formatBalance(t.reward)} — ` +
        `${t.description.slice(0, 50)}...`
      );
    }
  }

  // ── 8. Simulate task completion ───────────────────────────────────────────
  // In production: poster accepts bid first (acceptBid extrinsic),
  // then worker submits result. Here we demonstrate the completeTask call.
  console.log(`\nDemonstrating completeTask (requires task to be Assigned)...`);
  const resultPayload = JSON.stringify({
    summary: "ClawChain uses a hybrid NPoS+BABE consensus...",
    wordCount: 198,
    format: "markdown",
    completedAt: new Date().toISOString(),
  });

  try {
    const completeHash = await market.completeTask(worker, taskId, resultPayload);
    console.log(`✅ Task completed! Block hash: ${completeHash}`);
  } catch (err) {
    // Expected to fail if task isn't in Assigned state yet
    const msg = err instanceof Error ? err.message : String(err);
    console.log(`  (skipped — task not Assigned yet: ${msg.split(":")[0]})`);
  }

  // ── 9. Disconnect ─────────────────────────────────────────────────────────
  await client.disconnect();
  console.log(`\n👋 Disconnected. Done!`);
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
