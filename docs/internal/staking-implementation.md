# NPoS Staking Implementation for ClawChain

## Overview
This document describes the addition of Nominated Proof-of-Stake (NPoS) staking and session management to the ClawChain Substrate runtime.

## Changes Made

### 1. Dependencies Added (Cargo.toml)

#### Workspace Dependencies
- `pallet-session` (44.0) - Session key management and validator rotation
- `pallet-authorship` (44.0) - Block author tracking for staking rewards
- `pallet-staking` (44.0) - Core NPoS staking functionality
- `pallet-staking-reward-curve` (11.0) - Inflation curve for rewards
- `pallet-offences` (44.0) - Offense tracking for slashing
- `pallet-bags-list` (44.0) - Efficient nominator ordering
- `pallet-election-provider-multi-phase` (44.0) - NPoS election algorithm
- `pallet-treasury` (44.0) - On-chain treasury for governance
- `pallet-sudo` (44.0) - Superuser for testnet administration
- `sp-staking` (40.0) - Staking primitives
- `sp-npos-elections` (40.0) - NPoS election primitives
- `frame-election-provider-support` (44.0) - Election provider framework

#### Version Alignment
To ensure compatibility, all pallets and primitives were aligned to version 44.x:
- Downgraded `pallet-balances` from 46.0 → 44.0
- Downgraded `pallet-grandpa` from 45.0 → 44.0
- Downgraded `pallet-transaction-payment` from 45.0 → 44.0
- Downgraded `frame-support`, `frame-system`, `frame-executive` from 45.x → 44.0
- Downgraded `sp-runtime` from 45.0 → 43.0
- Downgraded `sp-io` from 44.0 → 42.0

### 2. Runtime Configuration (runtime/src/lib.rs)

#### Staking Constants
```rust
pub const SESSION_LENGTH: BlockNumber = 100;  // 10 minutes at 6s blocks
pub const SESSIONS_PER_ERA: SessionIndex = 6;  // 1 hour per era (testnet)
pub const MIN_VALIDATOR_BOND: u128 = 10_000 * 1_000_000_000_000;  // 10,000 CLAW
pub const MIN_NOMINATOR_BOND: u128 = 100 * 1_000_000_000_000;     // 100 CLAW
```

#### Session Keys
Updated `opaque::SessionKeys` to include both Aura and Grandpa:
```rust
pub struct SessionKeys {
    pub aura: Aura,
    pub grandpa: Grandpa,
}
```

#### Pallet Configurations
- **Authorship**: Tracks block authors for reward distribution
- **Session**: Manages validator sessions with 100-block periods
- **Historical**: Stores historical validator data for slashing
- **Staking**: Core NPoS with 10% annual inflation via reward curve
- **BagsList**: Efficient nominator selection with custom thresholds
- **ElectionProviderMultiPhase**: Off-chain + on-chain election process
- **Offences**: Handles validator misbehavior
- **Treasury**: Receives 20% of block rewards (via RewardRemainder)
- **Sudo**: Temporary superuser for testnet management

#### Reward Distribution
- 10% annual inflation via `REWARD_CURVE`
- Treasury receives remainder rewards (slashed funds, unclaimed rewards)
- Validators and nominators receive era payouts

### 3. Genesis Configuration (node/src/chain_spec.rs)

#### Updated Authority Keys
Changed from `(AuraId, GrandpaId)` to `(stash, controller, AuraId, GrandpaId)`:
```rust
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s),
    )
}
```

#### Genesis State
- **Session keys**: Maps validators to their session keys
- **Staking**: Initial validators (Alice, Bob) with 1M CLAW staked each
- **Invulnerables**: Protects initial validators from slashing (testnet only)
- **Sudo**: Alice as initial sudo key

### 4. Runtime Macro Updates

Added new pallets to `construct_runtime!`:
```rust
Session: pallet_session,
Historical: pallet_session::historical,
Authorship: pallet_authorship,
Staking: pallet_staking,
Offences: pallet_offences,
BagsList: pallet_bags_list,
ElectionProviderMultiPhase: pallet_election_provider_multi_phase,
Treasury: pallet_treasury,
Sudo: pallet_sudo,
```

## Configuration Details

### Session Management
- **Session length**: 100 blocks (~10 minutes)
- **Era length**: 6 sessions (~1 hour)
- **Bonding duration**: 28 eras (~28 hours)
- **Slash defer**: 27 eras

### Staking Parameters
- **Minimum validator bond**: 10,000 CLAW
- **Minimum nominator bond**: 100 CLAW
- **Maximum nominators per validator**: 64
- **Maximum exposure page size**: 64
- **History depth**: 84 eras (~3.5 days)

### Election
- **Signed phase**: 50 blocks (~5 minutes)
- **Unsigned phase**: 50 blocks (~5 minutes)
- **Maximum voters**: 10,000
- **Maximum electable targets**: 256
- **Maximum validators**: 100

### Treasury
- **Spend period**: 24 hours
- **Burn rate**: 0% (testnet)
- **Maximum approvals**: 100

## Known Issues & Next Steps

### Current Status
The implementation is complete but requires final testing:
1. Full workspace compilation to verify WASM build
2. Runtime tests to ensure pallet integration
3. Test network deployment to verify staking functionality

### Version Compatibility
All dependencies were aligned to version 44.x to ensure compatibility in WASM builds. The older pallet-balances (44.0 vs 46.0) may have fewer features but maintains compatibility.

### Future Enhancements
1. **Mainnet configuration**: Longer eras (24h), higher bonds, remove sudo
2. **Governance**: Add democracy/council pallets for treasury spending
3. **Nomination pools**: Allow smaller holders to participate
4. **Advanced slashing**: Configure specific slash amounts for offense types

## Testing Checklist
- [ ] `cargo check --workspace` passes
- [ ] `cargo test -p clawchain-runtime` passes  
- [ ] WASM build completes without errors
- [ ] Dev node starts with Alice as validator
- [ ] Staking extrinsics work (bond, nominate, validate)
- [ ] Era transitions occur correctly
- [ ] Rewards are distributed properly

## References
- [Substrate Staking Pallet](https://paritytech.github.io/substrate/master/pallet_staking/)
- [NPoS Overview](https://wiki.polkadot.network/docs/learn-phragmen)
- [Session Management](https://paritytech.github.io/substrate/master/pallet_session/)
