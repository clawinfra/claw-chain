/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  // Transpile @polkadot packages which use ESM
  transpilePackages: ['@polkadot/api', '@polkadot/util', '@polkadot/util-crypto', '@polkadot/types'],
  webpack: (config) => {
    // Required for @polkadot/api WASM support
    config.experiments = { ...config.experiments, asyncWebAssembly: true };
    return config;
  },
};

module.exports = nextConfig;
