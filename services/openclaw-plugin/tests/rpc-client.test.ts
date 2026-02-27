import { ClawChainRpcClient } from '../src/rpc/client';
import { WsProvider, ApiPromise, mockApi, mockProvider } from '@polkadot/api';

describe('ClawChainRpcClient', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Reset connected state
    mockApi.isConnected = true;
    mockProvider.on.mockImplementation((event: string, cb: () => void) => {
      if (event === 'connected') setImmediate(cb);
    });
  });

  describe('constructor', () => {
    it('should create client with default timeout', () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      expect(client.isConnected).toBe(false);
    });

    it('should create client with custom timeout', () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944', connectTimeoutMs: 5000 });
      expect(client.isConnected).toBe(false);
    });
  });

  describe('connect', () => {
    it('should connect and return API instance', async () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      const api = await client.connect();

      expect(WsProvider).toHaveBeenCalledWith('ws://localhost:9944', false);
      expect(ApiPromise.create).toHaveBeenCalledWith({ provider: mockProvider });
      expect(api).toBe(mockApi);
    });

    it('should return existing connection on second call', async () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      const api1 = await client.connect();
      const api2 = await client.connect();

      expect(ApiPromise.create).toHaveBeenCalledTimes(1);
      expect(api1).toBe(api2);
    });

    it('should reconnect if disconnected', async () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      await client.connect();

      // Simulate disconnection
      mockApi.isConnected = false;
      const api2 = await client.connect();

      expect(ApiPromise.create).toHaveBeenCalledTimes(2);
      expect(api2).toBe(mockApi);
    });

    it('should reject on connection timeout', async () => {
      jest.useFakeTimers();

      mockProvider.on.mockImplementation(() => {
        // Never fire connected
      });

      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944', connectTimeoutMs: 100 });
      const connectPromise = client.connect();

      jest.advanceTimersByTime(200);

      await expect(connectPromise).rejects.toThrow('RPC connection timed out after 100ms');
      jest.useRealTimers();
    });

    it('should reject on provider error', async () => {
      mockProvider.on.mockImplementation((event: string, cb: (err?: Error) => void) => {
        if (event === 'error') setImmediate(() => cb(new Error('Connection refused')));
      });

      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      await expect(client.connect()).rejects.toThrow('RPC provider error: Connection refused');
    });
  });

  describe('disconnect', () => {
    it('should disconnect cleanly', async () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      await client.connect();
      await client.disconnect();

      expect(mockApi.disconnect).toHaveBeenCalled();
      expect(client.isConnected).toBe(false);
    });

    it('should be a no-op if not connected', async () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      await expect(client.disconnect()).resolves.toBeUndefined();
    });
  });

  describe('getApi', () => {
    it('should return API when connected', async () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      await client.connect();
      expect(client.getApi()).toBe(mockApi);
    });

    it('should throw when not connected', () => {
      const client = new ClawChainRpcClient({ rpcUrl: 'ws://localhost:9944' });
      expect(() => client.getApi()).toThrow('Not connected to ClawChain node');
    });
  });
});
