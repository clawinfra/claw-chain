import { OpenClawPlugin } from '../src/plugin';
import { mockApi, mockQuery, mockTx, WsProvider, ApiPromise } from '@polkadot/api';
import { mockKeypair } from '@polkadot/keyring';
import * as fs from 'fs';

jest.mock('fs');
const mockReadFileSync = fs.readFileSync as jest.MockedFunction<typeof fs.readFileSync>;

const TEST_MNEMONIC = 'test word word word word word word word word word word word';
const TEST_ADDRESS = mockKeypair.address;
const TEST_DID = `did:claw:${TEST_ADDRESS}`;

const BASE_CONFIG = {
  rpcUrl: 'ws://localhost:9944',
  keypairPath: '/path/to/keypair',
  connectTimeoutMs: 30_000,
};

function setupConnectedMocks() {
  mockReadFileSync.mockReturnValue(TEST_MNEMONIC as any);

  // DID already registered
  mockQuery.agentRegistry.agentRegistry.mockResolvedValue({
    isNone: false,
    isEmpty: false,
    toJSON: () => ({ did: TEST_DID, registeredAt: 100 }),
  });
}

describe('OpenClawPlugin', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    setupConnectedMocks();
  });

  describe('initialize', () => {
    it('should initialize and register DID', async () => {
      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();

      expect(plugin.isInitialized).toBe(true);
      expect(ApiPromise.create).toHaveBeenCalled();
    });

    it('should be idempotent (second call is no-op)', async () => {
      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();
      await plugin.initialize();

      expect(ApiPromise.create).toHaveBeenCalledTimes(1);
    });

    it('should handle fresh registration (not yet on-chain)', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({ isNone: true, isEmpty: true });

      const mockTxResult = {
        signAndSend: jest.fn().mockImplementation((_pair: unknown, cb: (args: any) => void) => {
          setImmediate(() => cb({
            status: { isInBlock: true, asInBlock: { toString: () => '0xblock' } },
            dispatchError: undefined,
            txHash: { toString: () => '0xtx' },
          }));
          return Promise.resolve(() => {});
        }),
      };
      mockTx.agentRegistry.registerAgent.mockReturnValue(mockTxResult);

      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();
      expect(plugin.isInitialized).toBe(true);
    });
  });

  describe('handleStatusCommand', () => {
    it('should return error if not initialized', async () => {
      const plugin = new OpenClawPlugin(BASE_CONFIG);
      const result = await plugin.handleStatusCommand();
      expect(result.success).toBe(false);
      expect(result.error).toContain('not initialized');
    });

    it('should return full status after initialization', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ did: TEST_DID, registeredAt: 100 }),
      });
      mockQuery.gasQuota.agentQuotas.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ available: 1000, used: 100, resetAt: 9999 }),
      });
      mockQuery.reputation.reputations.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ score: 900, tier: 'Diamond', updatedAt: 200 }),
      });

      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();

      const result = await plugin.handleStatusCommand();
      expect(result.success).toBe(true);
      expect(result.command).toBe('clawchain_status');
      const data = result.data as any;
      expect(data.did.registered).toBe(true);
      expect(data.gasQuota.available).toBe('1000');
      expect(data.reputation.score).toBe(900);
      expect(data.queriedAt).toBeDefined();
    });

    it('should return error result when query throws', async () => {
      // Initialize first
      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();

      // Then make queries fail
      mockQuery.agentRegistry.agentRegistry.mockRejectedValue(new Error('RPC timeout'));

      const result = await plugin.handleStatusCommand();
      expect(result.success).toBe(false);
      expect(result.error).toContain('RPC timeout');
    });
  });

  describe('handleCommand', () => {
    it('should dispatch clawchain_status command', async () => {
      mockQuery.gasQuota.agentQuotas.mockResolvedValue({ isNone: true, isEmpty: true });
      mockQuery.reputation.reputations.mockResolvedValue({ isNone: true, isEmpty: true });

      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();

      const result = await plugin.handleCommand('clawchain_status');
      expect(result.command).toBe('clawchain_status');
    });

    it('should return error for unknown command', async () => {
      const plugin = new OpenClawPlugin(BASE_CONFIG);
      const result = await plugin.handleCommand('unknown_cmd');
      expect(result.success).toBe(false);
      expect(result.error).toContain('Unknown command: unknown_cmd');
    });
  });

  describe('shutdown', () => {
    it('should disconnect on shutdown', async () => {
      const plugin = new OpenClawPlugin(BASE_CONFIG);
      await plugin.initialize();
      await plugin.shutdown();

      expect(mockApi.disconnect).toHaveBeenCalled();
      expect(plugin.isInitialized).toBe(false);
    });
  });
});
