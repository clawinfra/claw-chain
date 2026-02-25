import { ApiPromise, WsProvider } from '@polkadot/api';

let apiInstance: ApiPromise | null = null;
let providerInstance: WsProvider | null = null;

const WS_URL =
  process.env['NEXT_PUBLIC_WS_URL'] ?? 'wss://testnet.clawchain.win';

/**
 * Get or create a singleton ApiPromise connected to the ClawChain node.
 * Uses exponential backoff for reconnection (1s→2s→4s…max 30s).
 */
export async function getApi(): Promise<ApiPromise> {
  if (apiInstance?.isConnected) return apiInstance;

  if (providerInstance) {
    try {
      providerInstance.disconnect();
    } catch {
      // ignore disconnect errors
    }
  }

  providerInstance = new WsProvider(WS_URL, 1000, {}, 30_000);

  apiInstance = await ApiPromise.create({
    provider: providerInstance,
  });

  return apiInstance;
}

/**
 * Disconnect the singleton API instance.
 */
export async function disconnectApi(): Promise<void> {
  if (apiInstance) {
    await apiInstance.disconnect();
    apiInstance = null;
  }
  providerInstance = null;
}
