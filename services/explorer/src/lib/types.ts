export interface BlockSummary {
  hash: string;
  number: number;
  timestamp: number;
  extrinsicCount: number;
  producer: string;
}

export interface ExtrinsicSummary {
  hash: string;
  index: number;
  section: string;
  method: string;
  signer: string | null;
  success: boolean;
}

export interface BlockInfo extends BlockSummary {
  parentHash: string;
  stateRoot: string;
  extrinsicsRoot: string;
  extrinsics: ExtrinsicSummary[];
}

export interface DecodedEvent {
  section: string;
  method: string;
  data: Record<string, unknown>;
}

export interface ExtrinsicInfo extends ExtrinsicSummary {
  blockHash: string;
  blockNumber: number;
  args: Record<string, unknown>;
  events: DecodedEvent[];
  tip: string;
  fee: string;
}

export interface AgentInfo {
  address: string;
  did: string | null;
  reputation: number | null;
  reputationHistory: { block: number; score: number }[];
  gasQuota: {
    remaining: string;
    total: string;
    lastRefill: number;
  } | null;
}

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';
