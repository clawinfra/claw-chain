import { useApiContext } from '@/providers/ApiProvider';

/**
 * Convenience hook to access the polkadot ApiPromise, connection status, and current block number.
 */
export function useApi() {
  return useApiContext();
}
