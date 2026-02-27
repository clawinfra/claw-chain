/**
 * Status Checker
 * Queries ClawChain for agent DID registration, gas quota, and reputation.
 */

import { ApiPromise } from '@polkadot/api';

export interface AgentStatus {
  did: {
    registered: boolean;
    identifier: string | null;
    registeredAt: number | null;
  };
  gasQuota: {
    available: string | null;
    used: string | null;
    resetAt: number | null;
  };
  reputation: {
    score: number | null;
    tier: string | null;
    updatedAt: number | null;
  };
  queriedAt: string;
}

export class StatusChecker {
  private readonly api: ApiPromise;

  constructor(api: ApiPromise) {
    this.api = api;
  }

  /**
   * Query DID registration status from agentRegistry pallet.
   */
  async queryDIDStatus(agentId: string): Promise<AgentStatus['did']> {
    const result = await (this.api.query as any).agentRegistry.agentRegistry(agentId);

    if (!result || result.isNone || result.isEmpty) {
      return { registered: false, identifier: null, registeredAt: null };
    }

    const entry = result.toJSON() as {
      did?: string;
      registeredAt?: number;
    };

    return {
      registered: true,
      identifier: entry.did ?? agentId,
      registeredAt: entry.registeredAt ?? null,
    };
  }

  /**
   * Query gas quota from gasQuota pallet.
   */
  async queryGasQuota(accountId: string): Promise<AgentStatus['gasQuota']> {
    const result = await (this.api.query as any).gasQuota.agentQuotas(accountId);

    if (!result || result.isNone || result.isEmpty) {
      return { available: null, used: null, resetAt: null };
    }

    const quota = result.toJSON() as {
      available?: string | number;
      used?: string | number;
      resetAt?: number;
    };

    return {
      available: quota.available != null ? String(quota.available) : null,
      used: quota.used != null ? String(quota.used) : null,
      resetAt: quota.resetAt ?? null,
    };
  }

  /**
   * Query reputation score from reputation pallet.
   */
  async queryReputation(accountId: string): Promise<AgentStatus['reputation']> {
    const result = await (this.api.query as any).reputation.reputations(accountId);

    if (!result || result.isNone || result.isEmpty) {
      return { score: null, tier: null, updatedAt: null };
    }

    const rep = result.toJSON() as {
      score?: number;
      tier?: string;
      updatedAt?: number;
    };

    return {
      score: rep.score ?? null,
      tier: rep.tier ?? null,
      updatedAt: rep.updatedAt ?? null,
    };
  }

  /**
   * Aggregate all status queries into a single response.
   * @param agentId - The DID or account ID to query
   * @param accountId - The SS58 account address (may differ from DID)
   */
  async getFullStatus(agentId: string, accountId: string): Promise<AgentStatus> {
    const [did, gasQuota, reputation] = await Promise.all([
      this.queryDIDStatus(agentId),
      this.queryGasQuota(accountId),
      this.queryReputation(accountId),
    ]);

    return {
      did,
      gasQuota,
      reputation,
      queriedAt: new Date().toISOString(),
    };
  }
}
