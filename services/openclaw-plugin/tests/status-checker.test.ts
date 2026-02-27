import { StatusChecker } from '../src/status/checker';
import { mockApi, mockQuery } from '@polkadot/api';

const AGENT_ID = 'did:claw:5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const ACCOUNT_ID = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

const emptyResult = { isNone: true, isEmpty: true };

describe('StatusChecker', () => {
  let checker: StatusChecker;

  beforeEach(() => {
    jest.clearAllMocks();
    checker = new StatusChecker(mockApi as any);
  });

  describe('queryDIDStatus', () => {
    it('should return unregistered for empty result', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue(emptyResult);
      const status = await checker.queryDIDStatus(AGENT_ID);
      expect(status.registered).toBe(false);
      expect(status.identifier).toBeNull();
    });

    it('should return registered with data', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ did: AGENT_ID, registeredAt: 500 }),
      });

      const status = await checker.queryDIDStatus(AGENT_ID);
      expect(status.registered).toBe(true);
      expect(status.identifier).toBe(AGENT_ID);
      expect(status.registeredAt).toBe(500);
    });

    it('should handle null result', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue(null);
      const status = await checker.queryDIDStatus(AGENT_ID);
      expect(status.registered).toBe(false);
    });
  });

  describe('queryGasQuota', () => {
    it('should return nulls for empty result', async () => {
      mockQuery.gasQuota.agentQuotas.mockResolvedValue(emptyResult);
      const quota = await checker.queryGasQuota(ACCOUNT_ID);
      expect(quota.available).toBeNull();
      expect(quota.used).toBeNull();
      expect(quota.resetAt).toBeNull();
    });

    it('should return quota data', async () => {
      mockQuery.gasQuota.agentQuotas.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ available: 1000, used: 250, resetAt: 9999 }),
      });

      const quota = await checker.queryGasQuota(ACCOUNT_ID);
      expect(quota.available).toBe('1000');
      expect(quota.used).toBe('250');
      expect(quota.resetAt).toBe(9999);
    });

    it('should handle null result', async () => {
      mockQuery.gasQuota.agentQuotas.mockResolvedValue(null);
      const quota = await checker.queryGasQuota(ACCOUNT_ID);
      expect(quota.available).toBeNull();
    });
  });

  describe('queryReputation', () => {
    it('should return nulls for empty result', async () => {
      mockQuery.reputation.reputations.mockResolvedValue(emptyResult);
      const rep = await checker.queryReputation(ACCOUNT_ID);
      expect(rep.score).toBeNull();
      expect(rep.tier).toBeNull();
    });

    it('should return reputation data', async () => {
      mockQuery.reputation.reputations.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ score: 750, tier: 'Gold', updatedAt: 12345 }),
      });

      const rep = await checker.queryReputation(ACCOUNT_ID);
      expect(rep.score).toBe(750);
      expect(rep.tier).toBe('Gold');
      expect(rep.updatedAt).toBe(12345);
    });

    it('should handle null result', async () => {
      mockQuery.reputation.reputations.mockResolvedValue(null);
      const rep = await checker.queryReputation(ACCOUNT_ID);
      expect(rep.score).toBeNull();
    });
  });

  describe('getFullStatus', () => {
    it('should aggregate all three queries', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ did: AGENT_ID, registeredAt: 100 }),
      });
      mockQuery.gasQuota.agentQuotas.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ available: 500, used: 50, resetAt: 9000 }),
      });
      mockQuery.reputation.reputations.mockResolvedValue({
        isNone: false,
        isEmpty: false,
        toJSON: () => ({ score: 800, tier: 'Platinum', updatedAt: 200 }),
      });

      const status = await checker.getFullStatus(AGENT_ID, ACCOUNT_ID);
      expect(status.did.registered).toBe(true);
      expect(status.gasQuota.available).toBe('500');
      expect(status.reputation.score).toBe(800);
      expect(status.queriedAt).toBeDefined();

      // All three queries should run in parallel
      expect(mockQuery.agentRegistry.agentRegistry).toHaveBeenCalledWith(AGENT_ID);
      expect(mockQuery.gasQuota.agentQuotas).toHaveBeenCalledWith(ACCOUNT_ID);
      expect(mockQuery.reputation.reputations).toHaveBeenCalledWith(ACCOUNT_ID);
    });

    it('should return nulls when chain has no data', async () => {
      mockQuery.agentRegistry.agentRegistry.mockResolvedValue(emptyResult);
      mockQuery.gasQuota.agentQuotas.mockResolvedValue(emptyResult);
      mockQuery.reputation.reputations.mockResolvedValue(emptyResult);

      const status = await checker.getFullStatus(AGENT_ID, ACCOUNT_ID);
      expect(status.did.registered).toBe(false);
      expect(status.gasQuota.available).toBeNull();
      expect(status.reputation.score).toBeNull();
    });
  });
});
