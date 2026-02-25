import { describe, it, expect, vi, beforeEach } from 'vitest';

// Use vi.hoisted so variables are available inside the vi.mock factory (which is hoisted)
const mockApiInstance = vi.hoisted(() => ({
  isConnected: true,
  disconnect: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@polkadot/api', () => ({
  ApiPromise: {
    create: vi.fn().mockResolvedValue(mockApiInstance),
  },
  WsProvider: vi.fn().mockImplementation(() => ({
    disconnect: vi.fn(),
  })),
}));

import { getApi, disconnectApi } from '@/lib/api';
import { ApiPromise, WsProvider } from '@polkadot/api';

describe('lib/api', () => {
  beforeEach(async () => {
    // Reset singleton state before each test
    await disconnectApi();
    vi.clearAllMocks();
    // Restore mock implementations cleared by clearAllMocks
    (ApiPromise.create as ReturnType<typeof vi.fn>).mockResolvedValue(mockApiInstance);
    mockApiInstance.isConnected = true;
    mockApiInstance.disconnect.mockResolvedValue(undefined);
  });

  describe('getApi', () => {
    it('creates and returns an ApiPromise instance', async () => {
      const api = await getApi();
      expect(api).toBeDefined();
      expect(ApiPromise.create).toHaveBeenCalledOnce();
    });

    it('creates a WsProvider with the WS URL', async () => {
      await getApi();
      expect(WsProvider).toHaveBeenCalled();
    });

    it('returns cached instance when already connected', async () => {
      const api1 = await getApi();
      const api2 = await getApi();
      // Should return same object, create only called once
      expect(api1).toBe(api2);
      expect(ApiPromise.create).toHaveBeenCalledTimes(1);
    });

    it('reconnects when existing api is not connected', async () => {
      const api1 = await getApi();
      // Simulate disconnection
      mockApiInstance.isConnected = false;
      const api2 = await getApi();
      expect(api2).toBeDefined();
      // ApiPromise.create called again for reconnection
      expect(ApiPromise.create).toHaveBeenCalledTimes(2);
    });
  });

  describe('disconnectApi', () => {
    it('disconnects the api instance', async () => {
      const api = await getApi();
      await disconnectApi();
      expect(api.disconnect).toHaveBeenCalledOnce();
    });

    it('handles disconnect when no instance exists', async () => {
      // Singleton was cleared in beforeEach, no instance
      await expect(disconnectApi()).resolves.toBeUndefined();
    });

    it('after disconnect, next getApi creates a fresh instance', async () => {
      await getApi();
      await disconnectApi();
      vi.clearAllMocks();
      (ApiPromise.create as ReturnType<typeof vi.fn>).mockResolvedValue(mockApiInstance);
      mockApiInstance.isConnected = true;

      const newApi = await getApi();
      expect(newApi).toBeDefined();
      expect(ApiPromise.create).toHaveBeenCalledTimes(1);
    });
  });
});
