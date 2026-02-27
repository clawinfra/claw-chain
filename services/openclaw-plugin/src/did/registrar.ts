/**
 * DID Registrar
 * Handles agent DID registration on-chain via agentRegistry pallet.
 * Signs the DID binding with the agent's ed25519 keypair.
 */

import { ApiPromise } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { KeyringPair } from '@polkadot/keyring/types';
import { readFileSync } from 'fs';

export interface KeypairConfig {
  /** Path to a JSON file containing the keypair seed/mnemonic, or a raw mnemonic string */
  keypairPath: string;
}

export interface DIDRegistrationResult {
  did: string;
  accountId: string;
  txHash: string;
  blockHash: string;
  success: boolean;
  alreadyRegistered: boolean;
}

export interface RegistrationStatus {
  registered: boolean;
  did: string | null;
  registeredAt: number | null;
  metadata: Record<string, unknown> | null;
}

export class DIDRegistrar {
  private readonly api: ApiPromise;
  private keypair: KeyringPair | null = null;
  private readonly keypairPath: string;

  constructor(api: ApiPromise, config: KeypairConfig) {
    this.api = api;
    this.keypairPath = config.keypairPath;
  }

  /**
   * Load the ed25519 keypair from file or mnemonic.
   */
  loadKeypair(): KeyringPair {
    if (this.keypair) return this.keypair;

    const keyring = new Keyring({ type: 'ed25519' });

    let seed: string;
    try {
      const raw = readFileSync(this.keypairPath, 'utf-8').trim();
      // Support both plain mnemonic/seed strings and JSON key files
      if (raw.startsWith('{')) {
        const json = JSON.parse(raw) as { mnemonic?: string; seed?: string; secretPhrase?: string };
        seed = json.mnemonic ?? json.seed ?? json.secretPhrase ?? '';
        if (!seed) {
          throw new Error('Keypair JSON file must contain "mnemonic", "seed", or "secretPhrase" field');
        }
      } else {
        seed = raw;
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
        throw new Error(`Keypair file not found: ${this.keypairPath}`);
      }
      throw err;
    }

    this.keypair = keyring.addFromUri(seed);
    return this.keypair;
  }

  /**
   * Derive the DID from the agent's account ID.
   * Format: did:claw:<hex-account-id>
   */
  deriveDID(accountId?: string): string {
    const pair = this.loadKeypair();
    const id = accountId ?? pair.address;
    return `did:claw:${id}`;
  }

  /**
   * Check if the agent DID is already registered on-chain.
   */
  async getRegistrationStatus(did: string): Promise<RegistrationStatus> {
    const result = await (this.api.query as any).agentRegistry.agentRegistry(did);

    if (result.isNone || result.isEmpty) {
      return { registered: false, did, registeredAt: null, metadata: null };
    }

    const entry = result.toJSON() as {
      did?: string;
      registeredAt?: number;
      metadata?: Record<string, unknown>;
    };

    return {
      registered: true,
      did: entry.did ?? did,
      registeredAt: entry.registeredAt ?? null,
      metadata: entry.metadata ?? null,
    };
  }

  /**
   * Register the agent DID on-chain via agentRegistry.registerAgent().
   * Signs with the agent's ed25519 keypair.
   * Idempotent — if already registered, returns success without re-submitting.
   */
  async registerDID(): Promise<DIDRegistrationResult> {
    const pair = this.loadKeypair();
    const did = this.deriveDID();

    // Check if already registered
    const status = await this.getRegistrationStatus(did);
    if (status.registered) {
      return {
        did,
        accountId: pair.address,
        txHash: '',
        blockHash: '',
        success: true,
        alreadyRegistered: true,
      };
    }

    return new Promise((resolve, reject) => {
      let resolved = false;

      (this.api.tx as any).agentRegistry
        .registerAgent(did)
        .signAndSend(pair, ({ status, dispatchError, txHash }: any) => {
          if (resolved) return;

          if (dispatchError) {
            resolved = true;
            if (dispatchError.isModule) {
              const decoded = this.api.registry.findMetaError(dispatchError.asModule);
              reject(new Error(`Dispatch error: ${decoded.section}.${decoded.name}: ${decoded.docs.join(' ')}`));
            } else {
              reject(new Error(`Dispatch error: ${dispatchError.toString()}`));
            }
            return;
          }

          if (status.isInBlock) {
            resolved = true;
            resolve({
              did,
              accountId: pair.address,
              txHash: txHash.toString(),
              blockHash: status.asInBlock.toString(),
              success: true,
              alreadyRegistered: false,
            });
          }
        })
        .catch((err: Error) => {
          if (!resolved) {
            resolved = true;
            reject(new Error(`Failed to submit registerAgent transaction: ${err.message}`));
          }
        });
    });
  }
}
