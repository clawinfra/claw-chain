//! ClawChain chain specifications.
//!
//! Defines the genesis configuration for development and testnet chains.

use clawchain_runtime::{
    opaque::SessionKeys, AccountId, Balance, Signature, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for ClawChain.
pub type ChainSpec = sc_service::GenericChainSpec;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate session keys for an authority.
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s),
    )
}

/// Helper function to create session keys.
pub fn session_keys(aura: AuraId, grandpa: GrandpaId) -> SessionKeys {
    SessionKeys { aura, grandpa }
}

/// ClawChain tokenomics constants.
/// Total supply: 1,000,000,000 CLAW (with 12 decimals)
const TOTAL_SUPPLY: u128 = 1_000_000_000 * 10u128.pow(12);
/// Airdrop allocation: 40%
const _AIRDROP_ALLOCATION: u128 = TOTAL_SUPPLY * 40 / 100;
/// Validator allocation: 30%
const VALIDATOR_ALLOCATION: u128 = TOTAL_SUPPLY * 30 / 100;
/// Treasury allocation: 20%
const TREASURY_ALLOCATION: u128 = TOTAL_SUPPLY * 20 / 100;
/// Team allocation: 10%
const _TEAM_ALLOCATION: u128 = TOTAL_SUPPLY * 10 / 100;

/// Staking: 1M CLAW for each validator (12 decimals)
const STASH: Balance = 1_000_000 * 10u128.pow(12);

/// Development chain spec — single authority (Alice), pre-funded accounts.
pub fn development_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
        None,
    )
    .with_name("ClawChain Development")
    .with_id("clawchain_dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_patch(testnet_genesis(
        // Initial authorities
        vec![authority_keys_from_seed("Alice")],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
        ],
        true,
    ))
    .with_protocol_id("clawchain")
    .build())
}

/// Local testnet chain spec — two authorities (Alice + Bob).
pub fn local_testnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
        None,
    )
    .with_name("ClawChain Local Testnet")
    .with_id("clawchain_local_testnet")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_patch(testnet_genesis(
        // Initial authorities
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
        ],
        true,
    ))
    .with_protocol_id("clawchain")
    .build())
}

/// Configure initial storage state for FRAME pallets.
fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, AuraId, GrandpaId)>,
    root_key: AccountId,
    mut endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> serde_json::Value {
    // Ensure initial validators are in endowed accounts
    initial_authorities.iter().for_each(|x| {
        if !endowed_accounts.contains(&x.0) {
            endowed_accounts.push(x.0.clone());
        }
        if !endowed_accounts.contains(&x.1) {
            endowed_accounts.push(x.1.clone());
        }
    });

    // Each endowed account gets an equal share of validator + treasury allocation for dev
    let endowment: u128 = (VALIDATOR_ALLOCATION + TREASURY_ALLOCATION)
        / endowed_accounts.len() as u128;

    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().map(|k| (k.clone(), endowment)).collect::<Vec<_>>(),
        },
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(), // stash
                        x.0.clone(), // stash
                        session_keys(x.2.clone(), x.3.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        },
        "staking": {
            "validatorCount": initial_authorities.len() as u32,
            "minimumValidatorCount": 1,
            "invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
            "stakers": initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(), // stash
                        x.1.clone(), // controller
                        STASH,
                        serde_json::json!("Validator"),
                    )
                })
                .collect::<Vec<_>>(),
        },
        "sudo": {
            "key": Some(root_key),
        },
    })
}
