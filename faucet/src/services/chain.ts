/**
 * Polkadot/Substrate chain interaction helpers.
 *
 * Handles API connection, balance queries, and CLAW transfers.
 * Uses @polkadot/api for all chain operations.
 */

import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import type { KeyringPair } from '@polkadot/keyring/types';

/**
 * Connect to the Substrate node at rpcUrl and return a ready ApiPromise.
 */
export async function connectChain(rpcUrl: string): Promise<ApiPromise> {
  const provider = new WsProvider(rpcUrl);
  const api = await ApiPromise.create({ provider });
  await api.isReady;
  return api;
}

/**
 * Build a Keyring pair from a seed phrase or dev URI (e.g. "//Alice").
 */
function buildKeyPair(seed: string): KeyringPair {
  const keyring = new Keyring({ type: 'sr25519' });
  return keyring.addFromUri(seed);
}

/**
 * Transfer `amount` planck from the faucet account to `toAddress`.
 * Returns the extrinsic hash (hex string) on success.
 * Throws on transaction failure.
 */
export async function transferClaw(
  api: ApiPromise,
  seed: string,
  toAddress: string,
  amount: bigint,
): Promise<string> {
  const pair = buildKeyPair(seed);

  return new Promise<string>((resolve, reject) => {
    let unsub: (() => void) | undefined;
    let settled = false;

    const settle = (fn: () => void): void => {
      if (settled) return;
      settled = true;
      if (unsub) unsub();
      fn();
    };

    api.tx.balances
      .transferKeepAlive(toAddress, amount)
      .signAndSend(pair, (result) => {
        const { status, dispatchError } = result;

        if (dispatchError) {
          let msg = 'Transfer failed';
          if (dispatchError.isModule) {
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            msg = `${decoded.section}.${decoded.name}: ${decoded.docs.join(' ')}`;
          } else {
            msg = dispatchError.toString();
          }
          settle(() => reject(new Error(msg)));
          return;
        }

        if (status.isInBlock || status.isFinalized) {
          settle(() => resolve(result.txHash.toHex()));
        } else if (status.isDropped || status.isInvalid || status.isUsurped) {
          settle(() => reject(new Error(`Transaction ${status.type.toLowerCase()}`)));
        }
      })
      .then((unsubFn) => {
        unsub = unsubFn;
      })
      .catch((err: unknown) => {
        settle(() => reject(err instanceof Error ? err : new Error(String(err))));
      });

    // 60-second timeout
    setTimeout(() => {
      settle(() => reject(new Error('Transaction timed out after 60 seconds')));
    }, 60_000);
  });
}

/**
 * Get the free balance of the faucet account in planck (as string).
 */
export async function getFaucetBalance(
  api: ApiPromise,
  seed: string,
): Promise<string> {
  const pair = buildKeyPair(seed);
  const account = await api.query.system.account(pair.address);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const free: bigint = (account as any).data.free.toBigInt();
  return free.toString();
}

/**
 * Get the faucet account's SS58 address.
 */
export function getFaucetAddress(seed: string): string {
  const pair = buildKeyPair(seed);
  return pair.address;
}
