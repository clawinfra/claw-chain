/**
 * @clawinfra/clawchain-sdk — TaskMarket
 *
 * High-level wrapper for the `pallet-task-market` extrinsics and storage.
 *
 * Extrinsics (writes — require a signer):
 *   - createTask(signer, description, reward) → taskId
 *   - bidOnTask(signer, taskId, proposal) → tx hash
 *   - completeTask(signer, taskId, result) → tx hash
 *
 * Queries (reads — free):
 *   - getTask(taskId) → TaskInfo | null
 *   - listTasks() → TaskInfo[]
 *   - listOpenTasks() → TaskInfo[]
 */

import type { KeyringPair } from "@polkadot/keyring/types";
import type { ClawChainClient } from "./client.js";
import type { TaskInfo, TaskStatus } from "./types.js";

/** Raw on-chain codec type returned by task-market storage query */
interface RawTaskInfo {
  poster: { toString(): string };
  description: { toHuman?(): unknown; toString(): string };
  reward: { toBigInt(): bigint };
  deadline: { toNumber(): number };
  status: { type: string };
  assignee?: { toString(): string } | null;
  postedAt: { toNumber(): number };
}

/**
 * TaskMarket — interact with the ClawChain `pallet-task-market`.
 *
 * ```ts
 * const client = new ClawChainClient("wss://testnet.clawchain.win");
 * await client.connect();
 *
 * const market = new TaskMarket(client);
 * const taskId = await market.createTask(
 *   signer,
 *   "Summarise this research paper and return JSON",
 *   BigInt("5000000000000")  // 5 CLAW
 * );
 * console.log("Task created! ID:", taskId);
 * ```
 */
export class TaskMarket {
  private readonly client: ClawChainClient;

  constructor(client: ClawChainClient) {
    this.client = client;
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Extrinsics (signed writes)
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Post a new task to the task market.
   *
   * Submits `taskMarket.postTask(description, reward, deadline)` and waits
   * for inclusion in a block. The `reward` amount in CLAW (Planck units) is
   * escrowed from the caller's account.
   *
   * @param signer       KeyringPair of the task poster
   * @param description  Human-readable task description (max 512 bytes)
   * @param reward       CLAW reward in Planck units (1 CLAW = 1e12)
   * @param deadline     Optional block-number deadline (0 = no deadline)
   * @returns Numeric task ID assigned by the chain
   */
  async createTask(
    signer: KeyringPair,
    description: string,
    reward: bigint,
    deadline = 0
  ): Promise<number> {
    const api = this.client.api;
    const descBytes = Array.from(Buffer.from(description, "utf8"));

    return new Promise((resolve, reject) => {
      let taskId: number | undefined;

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (api.tx as any).taskMarket
        .postTask(descBytes, reward.toString(), deadline)
        .signAndSend(
          signer,
          ({
            status,
            events,
            dispatchError,
          }: {
            status: { isInBlock: boolean };
            events: Array<{
              event: {
                section: string;
                method: string;
                data: { toJSON(): unknown[] };
              };
            }>;
            dispatchError?: { isModule: boolean; asModule: unknown };
          }) => {
            if (dispatchError) {
              if (dispatchError.isModule) {
                const decoded = api.registry.findMetaError(
                  dispatchError.asModule as Parameters<
                    typeof api.registry.findMetaError
                  >[0]
                );
                reject(
                  new Error(
                    `Transaction failed: ${decoded.section}.${decoded.name} — ${decoded.docs.join(" ")}`
                  )
                );
              } else {
                reject(
                  new Error(`Transaction failed: ${dispatchError.toString()}`)
                );
              }
              return;
            }

            if (status.isInBlock) {
              // Extract TaskPosted event to find the assigned taskId
              for (const { event } of events) {
                if (
                  event.section === "taskMarket" &&
                  event.method === "TaskPosted"
                ) {
                  const data = event.data.toJSON() as unknown[];
                  taskId = data[0] as number;
                }
              }
              if (taskId !== undefined) {
                resolve(taskId);
              } else {
                reject(
                  new Error(
                    "Task created but TaskPosted event not found. Check chain events."
                  )
                );
              }
            }
          }
        )
        .catch(reject);
    });
  }

  /**
   * Submit a bid on an open task.
   *
   * Calls `taskMarket.bidOnTask(taskId, proposal)`.
   *
   * @param signer    KeyringPair of the bidding agent
   * @param taskId    ID of the task to bid on
   * @param proposal  Free-text bid proposal / covering note (max 256 bytes)
   * @returns Hex-encoded extrinsic hash
   */
  async bidOnTask(
    signer: KeyringPair,
    taskId: number,
    proposal: string
  ): Promise<string> {
    const api = this.client.api;
    const proposalBytes = Array.from(Buffer.from(proposal, "utf8"));

    return new Promise((resolve, reject) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (api.tx as any).taskMarket
        .bidOnTask(taskId, proposalBytes)
        .signAndSend(signer, ({ status, dispatchError }: { status: { isInBlock: boolean; asInBlock: { toHex(): string } }; dispatchError?: { isModule: boolean; asModule: unknown } }) => {
          if (dispatchError) {
            if (dispatchError.isModule) {
              const decoded = api.registry.findMetaError(
                dispatchError.asModule as Parameters<
                  typeof api.registry.findMetaError
                >[0]
              );
              reject(
                new Error(
                  `Transaction failed: ${decoded.section}.${decoded.name} — ${decoded.docs.join(" ")}`
                )
              );
            } else {
              reject(
                new Error(`Transaction failed: ${dispatchError.toString()}`)
              );
            }
          }
          if (status.isInBlock) {
            resolve(status.asInBlock.toHex());
          }
        })
        .catch(reject);
    });
  }

  /**
   * Submit the result of a completed task.
   *
   * Calls `taskMarket.submitResult(taskId, result)`.
   * Task must be in `Assigned` status and caller must be the assignee.
   *
   * @param signer   KeyringPair of the assigned worker
   * @param taskId   ID of the task being completed
   * @param result   Completion proof or result payload (max 1 KB)
   * @returns Hex-encoded extrinsic hash
   */
  async completeTask(
    signer: KeyringPair,
    taskId: number,
    result: string
  ): Promise<string> {
    const api = this.client.api;
    const resultBytes = Array.from(Buffer.from(result, "utf8"));

    return new Promise((resolve, reject) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (api.tx as any).taskMarket
        .submitResult(taskId, resultBytes)
        .signAndSend(signer, ({ status, dispatchError }: { status: { isInBlock: boolean; asInBlock: { toHex(): string } }; dispatchError?: { isModule: boolean; asModule: unknown } }) => {
          if (dispatchError) {
            if (dispatchError.isModule) {
              const decoded = api.registry.findMetaError(
                dispatchError.asModule as Parameters<
                  typeof api.registry.findMetaError
                >[0]
              );
              reject(
                new Error(
                  `Transaction failed: ${decoded.section}.${decoded.name} — ${decoded.docs.join(" ")}`
                )
              );
            } else {
              reject(
                new Error(`Transaction failed: ${dispatchError.toString()}`)
              );
            }
          }
          if (status.isInBlock) {
            resolve(status.asInBlock.toHex());
          }
        })
        .catch(reject);
    });
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Queries (free reads)
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Fetch a single task by its numeric ID.
   *
   * @param taskId  On-chain task ID
   * @returns TaskInfo if found, null otherwise
   */
  async getTask(taskId: number): Promise<TaskInfo | null> {
    const api = this.client.api;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const rawOption = await (api.query as any).taskMarket.tasks(taskId);

    if (rawOption.isNone) return null;

    const raw: RawTaskInfo = rawOption.unwrap();
    return this._decodeTaskInfo(taskId, raw);
  }

  /**
   * List all tasks (all statuses) on-chain.
   */
  async listTasks(): Promise<TaskInfo[]> {
    const api = this.client.api;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const entries: [{ args: [{ toNumber(): number }] }, RawTaskInfo][] =
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (api.query as any).taskMarket.tasks.entries();

    return entries.map(([key, raw]) => {
      const taskId = key.args[0].toNumber();
      return this._decodeTaskInfo(taskId, raw);
    });
  }

  /**
   * Convenience: list only tasks with status "Open" (accepting bids).
   */
  async listOpenTasks(): Promise<TaskInfo[]> {
    const all = await this.listTasks();
    return all.filter((t) => t.status === "Open");
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Internal helpers
  // ────────────────────────────────────────────────────────────────────────────

  private _decodeTaskInfo(taskId: number, raw: RawTaskInfo): TaskInfo {
    const descHuman = raw.description.toHuman
      ? raw.description.toHuman()
      : raw.description.toString();
    const description = Array.isArray(descHuman)
      ? Buffer.from(descHuman as number[]).toString("utf8")
      : String(descHuman);

    const status = raw.status.type as TaskStatus;
    const assignee = raw.assignee ? raw.assignee.toString() : undefined;

    return {
      taskId,
      poster: raw.poster.toString(),
      description,
      reward: raw.reward.toBigInt(),
      deadline: raw.deadline.toNumber(),
      status,
      assignee,
      postedAt: raw.postedAt.toNumber(),
    };
  }
}
