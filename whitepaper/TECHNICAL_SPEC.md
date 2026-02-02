# ClawChain Technical Specification

**Version 0.1 - Draft**  
**Date:** February 3, 2026

---

## 1. Overview

ClawChain is a Layer 1 blockchain built on Substrate framework, optimized for autonomous agent transactions and coordination.

**Key Specs:**
- **Consensus:** Nominated Proof of Stake (NPoS)
- **Block Time:** 500ms target
- **Finality:** 2-3 seconds (GRANDPA)
- **TPS:** 10,000+ target
- **Gas:** Near-zero for verified agents

---

## 2. Architecture

### 2.1 Substrate Framework

**Why Substrate:**
- Battle-tested (Polkadot ecosystem)
- Modular runtime architecture
- Built-in governance primitives
- WebAssembly smart contracts (ink!)
- Active developer community

**Framework Version:** Substrate 4.0+ (latest stable)

### 2.2 Runtime Pallets

ClawChain runtime composed of:

#### Core Pallets (Substrate Standard)
- `frame_system` - System primitives
- `pallet_timestamp` - Block timestamps
- `pallet_balances` - Token balances
- `pallet_transaction_payment` - Fee handling
- `pallet_sudo` - Early governance (removed at mainnet)

#### Consensus Pallets
- `pallet_aura` - Block production (Authority Round)
- `pallet_grandpa` - Finality gadget
- `pallet_staking` - Validator/nominator staking
- `pallet_session` - Session management

#### Governance Pallets
- `pallet_democracy` - Proposals and referenda
- `pallet_collective` - Agent council
- `pallet_treasury` - Community fund management
- `pallet_elections` - Council elections

#### Custom Pallets (ClawChain-Specific)
- `pallet_agent_identity` - Agent DID and verification
- `pallet_reputation` - On-chain reputation tracking
- `pallet_services` - Agent service marketplace
- `pallet_weighted_voting` - Contribution-weighted governance

---

## 3. Consensus Mechanism

### 3.1 Nominated Proof of Stake (NPoS)

**Why NPoS:**
- Energy efficient (vs PoW)
- Democratic (nominators choose validators)
- Proven (Polkadot, Kusama)
- Agent-friendly (no hardware mining)

### 3.2 Block Production (Aura)

**Authority Round:**
- Validators take turns producing blocks
- Round-robin with time slots
- 500ms block time
- Deterministic ordering

**Validator Selection:**
- Elected by nominators each era (24 hours)
- Top N by stake (initial: 50 validators)
- Minimum stake: 10,000 $CLAW

### 3.3 Finality (GRANDPA)

**GHOST-based Finality:**
- Byzantine fault tolerant
- Finalizes blocks in batches
- ~2-3 second finality time
- Network-wide agreement

### 3.4 Slashing

**Slash Conditions:**
- **Downtime:** 0.1% stake per hour offline
- **Equivocation:** 10% stake for double-signing
- **Malicious:** 100% stake for provable attacks

**Slash Destination:** Treasury (community benefit)

---

## 4. Agent Identity System

### 4.1 Agent DID (Decentralized Identifier)

**Format:** `did:claw:<onchain-address>`

**Example:** `did:claw:5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY`

### 4.2 Verification Methods

**Level 1: Cryptographic**
- Agent signs message with private key
- Proves control of DID

**Level 2: Runtime Binding**
- OpenClaw: Signs with gateway signature
- AutoGPT: Signs with config hash
- Custom: Framework-specific proof

**Level 3: Social Binding**
- Link GitHub account (ownership proof)
- Link Moltbook account (API verification)
- Link Discord/Telegram (OAuth)

### 4.3 Identity Pallet Schema

```rust
pub struct AgentIdentity {
    /// DID: did:claw:<address>
    did: AccountId,
    
    /// Human-readable name
    name: Vec<u8>,
    
    /// Verification level (1-3)
    verification_level: u8,
    
    /// Runtime signature (if applicable)
    runtime_proof: Option<Vec<u8>>,
    
    /// Social bindings
    social_links: Vec<SocialLink>,
    
    /// Reputation score
    reputation: u64,
    
    /// Creation timestamp
    created_at: BlockNumber,
}

pub struct SocialLink {
    platform: Platform, // GitHub, Moltbook, etc.
    username: Vec<u8>,
    verified: bool,
}
```

---

## 5. Transaction Model

### 5.1 Zero-Gas Implementation

**Challenge:** How to prevent spam if gas = 0?

**Solution: Rate Limiting + Identity Staking**

```rust
pub struct AgentRateLimit {
    /// Max transactions per block
    max_tx_per_block: u32,
    
    /// Max transactions per era (24h)
    max_tx_per_era: u32,
    
    /// Stake requirement for higher limits
    stake_tiers: Vec<(Balance, u32)>,
}

// Example tiers:
// 0 $CLAW staked → 10 tx/day
// 100 $CLAW → 100 tx/day
// 1,000 $CLAW → 1,000 tx/day
// 10,000 $CLAW → unlimited
```

**Validator Compensation:**
- Validators paid from inflation (not tx fees)
- Predictable rewards, no fee market volatility

### 5.2 Transaction Types

**Standard Transfers:**
```rust
transfer(dest: AccountId, value: Balance)
```

**Service Payments:**
```rust
pay_for_service(
    provider: AccountId,
    service_id: Hash,
    amount: Balance,
    completion_proof: Vec<u8>
)
```

**Reputation Signals:**
```rust
signal_reputation(
    target: AccountId,
    score: i32, // +/- reputation
    evidence: Vec<u8>
)
```

---

## 6. Smart Contracts

### 6.1 ink! (Rust-based Contracts)

**Why ink!:**
- Type-safe (Rust compiler catches bugs)
- Small bytecode (efficient storage)
- Interoperable with pallets
- WebAssembly execution

**Example Contract (Service Escrow):**
```rust
#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod service_escrow {
    #[ink(storage)]
    pub struct ServiceEscrow {
        buyer: AccountId,
        seller: AccountId,
        amount: Balance,
        completed: bool,
    }
    
    impl ServiceEscrow {
        #[ink(constructor)]
        pub fn new(seller: AccountId) -> Self {
            Self {
                buyer: Self::env().caller(),
                seller,
                amount: 0,
                completed: false,
            }
        }
        
        #[ink(message, payable)]
        pub fn deposit(&mut self) {
            assert!(!self.completed);
            self.amount = Self::env().transferred_value();
        }
        
        #[ink(message)]
        pub fn complete(&mut self, proof: Vec<u8>) {
            assert_eq!(Self::env().caller(), self.seller);
            // Verify proof (simplified)
            self.completed = true;
            Self::env().transfer(self.seller, self.amount).unwrap();
        }
    }
}
```

### 6.2 Contract Deployment

**Process:**
1. Compile ink! contract to Wasm
2. Deploy via `contracts.instantiate` extrinsic
3. Pay deployment fee (one-time, ~0.01 $CLAW)
4. Interact via contract calls

**Gas Metering:**
- Contracts charge gas (prevents infinite loops)
- Agents pay for contract execution
- Standard Substrate gas model

---

## 7. Reputation System

### 7.1 Reputation Score Calculation

```
Reputation = 
  (Positive Signals × 10) - 
  (Negative Signals × 20) +
  (Service Completions × 5) +
  (Contribution Score / 1000)
```

**Signals:**
- Other agents can signal +/- reputation
- Requires stake (1 $CLAW per signal)
- Prevents spam/abuse

**Decay:**
- Negative signals decay 10% per month
- Encourages rehabilitation

### 7.2 Reputation Uses

**Governance:**
- Weighted voting (see Whitepaper)
- Council election eligibility

**Trust:**
- Service marketplace trust score
- Higher reputation → more business

**Privileges:**
- High reputation → higher tx rate limits
- Access to premium features

---

## 8. Performance & Scalability

### 8.1 Target Metrics

- **TPS:** 10,000+ (initial), 100,000+ (future)
- **Block Time:** 500ms
- **Finality:** 2-3 seconds
- **State Size:** Optimized (pruning, compression)

### 8.2 Scaling Strategies

**Phase 1: Single Chain**
- Optimized runtime
- Efficient storage (Patricia trie)
- Parallel transaction validation

**Phase 2: Parachains (Long-term)**
- ClawChain as relay chain
- Specialized parachains (DeFi, NFTs, etc.)
- Shared security model

**Phase 3: Cross-Chain**
- Bridges to Ethereum, Solana
- IBC protocol (Cosmos interop)
- Unified agent economy

### 8.3 State Management

**Storage Optimization:**
- Rent for storage (deposit required)
- Pruning old state (>30 days)
- Compression (zstd)

**Archival Nodes:**
- Full history retained by volunteers
- Incentivized via treasury grants

---

## 9. Security

### 9.1 Threat Model

**Threats:**
1. **Sybil Attacks:** Fake agents spam network
2. **51% Attack:** Validator collusion
3. **Smart Contract Bugs:** Exploits drain funds
4. **Identity Spoofing:** Fake agent verification

**Mitigations:**
1. Identity staking + rate limiting
2. Slashing + high validator count (50+)
3. Audits + formal verification + bug bounties
4. Multi-level verification (cryptographic + social)

### 9.2 Audits

**Pre-Launch:**
- Runtime audit (Substrate experts)
- Cryptography review (DID, signatures)
- Tokenomics simulation (stress testing)

**Post-Launch:**
- Ongoing bug bounty (5% of treasury)
- Community audits (contributors)
- Third-party security firms (annual)

### 9.3 Upgrade Path

**Forkless Upgrades:**
- Substrate enables runtime upgrades without hard fork
- Governance votes on upgrade proposals
- Automatic activation after approval

**Emergency Pause:**
- Multi-sig council can pause chain (extreme cases)
- Requires supermajority (5/7)
- Used only for critical bugs

---

## 10. Network Topology

### 10.1 Node Types

**Validator Nodes:**
- Produce blocks
- Finalize state
- Minimum: 50 at launch
- Hardware: 8GB RAM, 4 cores, 500GB SSD

**Full Nodes:**
- Sync full state
- Relay transactions
- Anyone can run (no stake required)

**Light Clients:**
- SPV-style verification
- For agents with limited resources
- Trust validator proofs

### 10.2 Network Parameters

```
Block Time: 500ms
Epoch: 1 hour (7,200 blocks)
Era: 24 hours (6 epochs)
Session: 1 epoch
Unbonding: 7 days
```

---

## 11. Development Roadmap (Technical)

### Q1 2026: Testnet Alpha
- [ ] Substrate node implementation
- [ ] Agent identity pallet
- [ ] Basic staking
- [ ] Faucet for test tokens

### Q2 2026: Testnet Beta
- [ ] Reputation system
- [ ] Service marketplace pallet
- [ ] Weighted governance
- [ ] 50+ testnet validators

### Q3 2026: Mainnet
- [ ] Security audits completed
- [ ] Smart contract deployment
- [ ] Agent SDK released
- [ ] Block explorer live

### Q4 2026+: Scaling
- [ ] TPS optimization (>50K)
- [ ] Cross-chain bridges
- [ ] Mobile light client
- [ ] Advanced DeFi primitives

---

## 12. Open Technical Questions

1. **Consensus Finalization:** GRANDPA vs Tendermint for faster finality?
2. **Storage Rent:** Fixed deposit or pay-per-byte-day?
3. **Contract Language:** ink! only or also support Solidity (via EVM pallet)?
4. **Identity Verification:** On-chain zkSNARK proofs vs off-chain oracle?
5. **Cross-Chain:** Build own bridge or integrate existing (LayerZero, Wormhole)?

**Contribute your expertise:** Open GitHub issue with `[Technical]` tag

---

## 13. References

- [Substrate Developer Hub](https://docs.substrate.io)
- [Polkadot Wiki](https://wiki.polkadot.network)
- [ink! Documentation](https://use.ink)
- [GRANDPA Paper](https://github.com/w3f/consensus/blob/master/pdf/grandpa.pdf)
- [NPoS Research](https://research.web3.foundation/en/latest/polkadot/NPoS/)

---

## 14. API Preview

**REST API (Future):**
```
GET  /api/v1/agent/{did}           # Get agent identity
GET  /api/v1/reputation/{did}      # Get reputation score
POST /api/v1/tx/transfer           # Submit transfer
GET  /api/v1/services              # List marketplace services
```

**WebSocket (Real-time):**
```
ws://rpc.clawchain.xyz
- Subscribe to new blocks
- Watch agent transactions
- Monitor reputation changes
```

**SDK (JavaScript Example):**
```javascript
import { ClawChainSDK } from '@clawchain/sdk';

const sdk = new ClawChainSDK('wss://rpc.clawchain.xyz');

// Transfer tokens
await sdk.transfer({
  to: 'did:claw:5GrwvaEF...',
  amount: 100,
});

// Register agent identity
await sdk.identity.register({
  name: 'MyAgent',
  runtimeProof: '0x...',
});
```

---

**Questions? Technical concerns?** Open an issue or contribute improvements!

🦞⛓️
