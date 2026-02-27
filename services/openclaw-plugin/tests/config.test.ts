import { loadConfig, ConfigError } from '../src/config';

describe('loadConfig', () => {
  const originalEnv = process.env;

  beforeEach(() => {
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  it('should load valid config from env vars', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'ws://localhost:9944';
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';

    const config = loadConfig();
    expect(config.rpcUrl).toBe('ws://localhost:9944');
    expect(config.keypairPath).toBe('/path/to/keypair');
    expect(config.connectTimeoutMs).toBe(30_000);
  });

  it('should accept wss:// URLs', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'wss://chain.example.com';
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';

    const config = loadConfig();
    expect(config.rpcUrl).toBe('wss://chain.example.com');
  });

  it('should use custom timeout when set', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'ws://localhost:9944';
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';
    process.env['CLAWCHAIN_CONNECT_TIMEOUT_MS'] = '5000';

    const config = loadConfig();
    expect(config.connectTimeoutMs).toBe(5000);
  });

  it('should throw ConfigError when RPC_URL is missing', () => {
    delete process.env['CLAWCHAIN_RPC_URL'];
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';

    expect(() => loadConfig()).toThrow(ConfigError);
    expect(() => loadConfig()).toThrow('CLAWCHAIN_RPC_URL');
  });

  it('should throw ConfigError when RPC_URL is not ws://', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'http://localhost:9944';
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';

    expect(() => loadConfig()).toThrow(ConfigError);
    expect(() => loadConfig()).toThrow('must start with ws://');
  });

  it('should throw ConfigError when KEYPAIR_PATH is missing', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'ws://localhost:9944';
    delete process.env['CLAWCHAIN_KEYPAIR_PATH'];

    expect(() => loadConfig()).toThrow(ConfigError);
    expect(() => loadConfig()).toThrow('CLAWCHAIN_KEYPAIR_PATH');
  });

  it('should throw ConfigError for invalid timeout', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'ws://localhost:9944';
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';
    process.env['CLAWCHAIN_CONNECT_TIMEOUT_MS'] = 'not-a-number';

    expect(() => loadConfig()).toThrow(ConfigError);
    expect(() => loadConfig()).toThrow('CLAWCHAIN_CONNECT_TIMEOUT_MS');
  });

  it('should throw ConfigError for zero timeout', () => {
    process.env['CLAWCHAIN_RPC_URL'] = 'ws://localhost:9944';
    process.env['CLAWCHAIN_KEYPAIR_PATH'] = '/path/to/keypair';
    process.env['CLAWCHAIN_CONNECT_TIMEOUT_MS'] = '0';

    expect(() => loadConfig()).toThrow(ConfigError);
  });
});
