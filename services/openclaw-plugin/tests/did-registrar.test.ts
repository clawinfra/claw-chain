import { DIDRegistrar } from '../src/did/registrar';
import { mockApi, mockQuery, mockTx } from '@polkadot/api';
import { mockKeypair } from '@polkadot/keyring';
import * as fs from 'fs';

jest.mock('fs');
const mockReadFileSync = fs.readFileSync as jest.MockedFunction<typeof fs.readFileSync>;

const TEST_MNEMONIC = 'test word word word word word word word word word word word';
const TEST_ADDRESS = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const TEST_DID = `did:claw:${TEST_ADDRESS}`;

describe('DIDRegistrar', () => {
  let registrar: DIDRegistrar;

  beforeEach(() => {
    jest.clearAllMocks();
    mockReadFileSync.mockReturnValue(TEST_MNEMONIC as any);
    registrar = new DIDRegistrar(mockApi as any, { keypairPath: '/path/to/keypair' });
  });

  describe('loadKeypair', () => {
    it('should load keypair from plain mnemonic file', () => {
      const pair = registrar.loadKeypair();
      expect(pair).toBe(mockKeypair);
    });

    it('should cache keypair on subsequent calls', () => {
      const pair1 = registrar.loadKeypair();
      const pair2 = registrar.loadKeypair();
      expect(pair1).toBe(pair2);
      expect(mockReadFileSync).toHaveBeenCalledTimes(1);
    });

    it('should load keypair from JSON file with mnemonic field', () => {
      mockReadFileSync.mockReturnValue(JSON.stringify({ mnemonic: TEST_MNEMONIC }) as any);
      const r = new DIDRegistrar(mockApi as any, { keypairPath: '/path/to/keypair.json' });
      const pair = r.loadKeypair();
      expect(pair).toBe(mockKeypair);
    });

    it('should load keypair from JSON file with seed field', () => {
      mockReadFileSync.mockReturnValue(JSON.stringify({ seed: TEST_MNEMONIC }) as any);
      const r = new DIDRegistrar(mockApi as any, { keypairPath: '/path/to/keypair.json' });
      const pair = r.loadKeypair();
      expect(pair).toBe(mockKeypair);
    });

    it('should load keypair from JSON file with secretPhrase field', () => {
      mockReadFileSync.mockReturnValue(JSON.stringify({ secretPhrase: TEST_MNEMONIC }) as any);
      const r = new DIDRegistrar(mockApi as any, { keypairPath: '/path/to/keypair.json' });
      const pair = r.loadKeypair();
      expect(pair).toBe(mockKeypair);
    });

    it('should throw if JSON file has no known seed field', () => {
      mockReadFileSync.mockReturnValue(JSON.stringify({ foo: 'bar' }) as any);
      const r = new DIDRegistrar(mockApi as any, { keypairPath: '/path/to/keypair.json' });
      expect(() => r.loadKeypair()).toThrow('Keypair JSON file must contain');
    });

    it('should throw if file not found', () => {
      const err = new Error('ENOENT') as NodeJS.ErrnoException;
      err.code = 'ENOENT';
      mockReadFileSync.mockImplementation(() => { throw err; });
      const r = new DIDRegistrar(mockApi as any, { keypairPath: '/missing/path' });
      expect(() => r.loadKeypair()).toThrow('Keypair file not found: /missing/path');
    });
  });

  describe('deriveDID', () => {
    it('should derive DID from keypair address', () => {
      const did = registrar.deriveDID();
      expect(did).toBe(TEST_DID);
    });

    it('should derive DID from explicit accountId', () => {
      const did = registrar.deriveDID('5FakeAddress');
      expect(did).toBe('did:claw:5FakeAddress');
    });
  });

  describe('getRegistrationStatus', () => {
    it('should return unregistered when result is empty', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({ isNone: true, isEmpty: true });

      const status = await registrar.getRegistrationStatus(TEST_DID);
      expect(status.registered).toBe(false);
      expect(status.did).toBe(TEST_DID);
    });

    it('should return registered with metadata when found', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ did: TEST_DID, registeredAt: 1000 }),
      });

      const status = await registrar.getRegistrationStatus(TEST_DID);
      expect(status.registered).toBe(true);
      expect(status.registeredAt).toBe(1000);
    });
  });

  describe('registerDID', () => {
    it('should skip registration if already registered', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ did: TEST_DID, registeredAt: 1000 }),
      });

      const result = await registrar.registerDID();
      expect(result.alreadyRegistered).toBe(true);
      expect(result.success).toBe(true);
      expect(result.did).toBe(TEST_DID);
    });

    it('should register DID when not yet on-chain', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({ isNone: true, isEmpty: true });

      const mockTxResult = {
        signAndSend: jest.fn().mockImplementation((_pair: unknown, cb: (args: any) => void) => {
          setImmediate(() => cb({
            status: { isInBlock: true, asInBlock: { toString: () => '0xblock123' } },
            dispatchError: undefined,
            txHash: { toString: () => '0xtx456' },
          }));
          return Promise.resolve(() => {});
        }),
      };

      mockTx.agentRegistry.registerAgent.mockReturnValue(mockTxResult);

      const result = await registrar.registerDID();
      expect(result.success).toBe(true);
      expect(result.alreadyRegistered).toBe(false);
      expect(result.txHash).toBe('0xtx456');
      expect(result.blockHash).toBe('0xblock123');
    });

    it('should reject on dispatch error (module error)', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({ isNone: true, isEmpty: true });

      const mockTxResult = {
        signAndSend: jest.fn().mockImplementation((_pair: unknown, cb: (args: any) => void) => {
          setImmediate(() => cb({
            status: {},
            dispatchError: {
              isModule: true,
              asModule: { section: 'agentRegistry', name: 'Error', docs: ['test'] },
            },
            txHash: { toString: () => '0x' },
          }));
          return Promise.resolve(() => {});
        }),
      };

      mockTx.agentRegistry.registerAgent.mockReturnValue(mockTxResult);

      await expect(registrar.registerDID()).rejects.toThrow('Dispatch error');
    });

    it('should reject on generic dispatch error', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({ isNone: true, isEmpty: true });

      const mockTxResult = {
        signAndSend: jest.fn().mockImplementation((_pair: unknown, cb: (args: any) => void) => {
          setImmediate(() => cb({
            status: {},
            dispatchError: {
              isModule: false,
              toString: () => 'BadOrigin',
            },
            txHash: { toString: () => '0x' },
          }));
          return Promise.resolve(() => {});
        }),
      };

      mockTx.agentRegistry.registerAgent.mockReturnValue(mockTxResult);

      await expect(registrar.registerDID()).rejects.toThrow('Dispatch error: BadOrigin');
    });

    it('should reject when signAndSend throws', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({ isNone: true, isEmpty: true });

      const mockTxResult = {
        signAndSend: jest.fn().mockRejectedValue(new Error('nonce too low')),
      };

      mockTx.agentRegistry.registerAgent.mockReturnValue(mockTxResult);

      await expect(registrar.registerDID()).rejects.toThrow('Failed to submit registerAgent transaction: nonce too low');
    });
  });
});
