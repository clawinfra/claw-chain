/**
 * Mock for @polkadot/keyring
 */

export const mockKeypair = {
  address: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  publicKey: new Uint8Array(32).fill(1),
  sign: jest.fn().mockReturnValue(new Uint8Array(64).fill(0)),
  verify: jest.fn().mockReturnValue(true),
};

export const Keyring = jest.fn().mockImplementation(() => ({
  addFromUri: jest.fn().mockReturnValue(mockKeypair),
  addFromJson: jest.fn().mockReturnValue(mockKeypair),
}));
