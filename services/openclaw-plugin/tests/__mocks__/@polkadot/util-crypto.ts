/**
 * Mock for @polkadot/util-crypto
 */

export const cryptoWaitReady = jest.fn().mockResolvedValue(true);
export const ed25519Sign = jest.fn().mockReturnValue(new Uint8Array(64).fill(0));
export const ed25519Verify = jest.fn().mockReturnValue(true);
export const mnemonicGenerate = jest.fn().mockReturnValue('test word word word word word word word word word word word');
export const mnemonicToMiniSecret = jest.fn().mockReturnValue(new Uint8Array(32).fill(1));
