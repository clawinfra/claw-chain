/**
 * Mock for @polkadot/api
 */

export const mockQuery = {
  agentRegistry: {
    agentRegistry: jest.fn(),
  },
  gasQuota: {
    agentQuotas: jest.fn(),
  },
  reputation: {
    reputations: jest.fn(),
  },
};

export const mockTx = {
  agentRegistry: {
    registerAgent: jest.fn(),
  },
};

export const mockRegistry = {
  findMetaError: jest.fn().mockReturnValue({
    section: 'agentRegistry',
    name: 'AlreadyRegistered',
    docs: ['Agent already registered'],
  }),
};

export const mockApi = {
  query: mockQuery,
  tx: mockTx,
  registry: mockRegistry,
  isConnected: true,
  isReady: Promise.resolve(true),
  disconnect: jest.fn().mockResolvedValue(undefined),
};

export const mockProvider = {
  connect: jest.fn(),
  disconnect: jest.fn(),
  on: jest.fn((event: string, cb: () => void) => {
    if (event === 'connected') {
      setImmediate(cb);
    }
  }),
  isConnected: true,
};

export const WsProvider = jest.fn().mockImplementation(() => mockProvider);

export const ApiPromise = {
  create: jest.fn().mockResolvedValue(mockApi),
};
