//! ClawChain chain specifications.
//!
//! Defines the genesis configuration for development, testnet, and mainnet chains.
//!
//! # Mainnet Configuration
//!
//! The mainnet spec reads authority configuration from a JSON file at runtime,
//! avoiding hardcoded seeds. Set the `CLAWCHAIN_AUTHORITIES_FILE` environment
//! variable or place `./authorities.json` in the working directory.
//!
//! ## Authority JSON Schema
//!
//! ```json
//! {
//!   "authorities": [
//!     {
//!       "name": "validator-1",
//!       "stash": "5Grw...",
//!       "controller": "5Grw...",
//!       "aura": "0xd435...",
//!       "grandpa": "0x88dc..."
//!     }
//!   ],
//!   "sudo": "5GrwvaEF...",
//!   "endowed_accounts": [
//!     { "account": "5Grw...", "balance": "500000000000000000000" }
//!   ]
//! }
//! ```

use clawchain_runtime::{opaque::SessionKeys, AccountId, Balance, Signature, WASM_BINARY};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::UncheckedFrom, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialised `ChainSpec` for ClawChain.
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

// ─── Tokenomics constants ────────────────────────────────────────────────────

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

// ─── Mainnet authority parsing ────────────────────────────────────────────────

/// Authority entry as read from the JSON authorities file.
#[derive(Debug, serde::Deserialize)]
struct AuthorityEntry {
    /// Human-readable name for this validator node.
    pub name: String,
    /// Stash account (SS58-encoded).
    pub stash: String,
    /// Controller account (SS58-encoded).
    pub controller: String,
    /// Aura (sr25519) public key, 0x-prefixed hex.
    pub aura: String,
    /// GRANDPA (ed25519) public key, 0x-prefixed hex.
    pub grandpa: String,
}

/// Endowed account entry as read from the JSON authorities file.
#[derive(Debug, serde::Deserialize)]
struct EndowedEntry {
    /// Account (SS58-encoded).
    pub account: String,
    /// Balance in planck (string to avoid JSON integer overflow).
    pub balance: String,
}

/// Top-level JSON schema for the authorities configuration file.
#[derive(Debug, serde::Deserialize)]
struct AuthoritiesConfig {
    pub authorities: Vec<AuthorityEntry>,
    pub sudo: String,
    pub endowed_accounts: Vec<EndowedEntry>,
}

/// Decode a 0x-prefixed (or bare) hex string to bytes.
///
/// Returns an error if the input contains invalid characters or has an odd length.
pub fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() % 2 != 0 {
        return Err(format!(
            "Hex string '{}' has odd length ({} chars); expected an even number",
            s,
            s.len()
        ));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex character at position {}: {}", i, e))
        })
        .collect()
}

/// Hex-encode bytes (no 0x prefix).
pub fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

/// Parse an SS58-encoded [`AccountId`].
pub fn parse_account_id(ss58: &str) -> Result<AccountId, String> {
    use sp_core::crypto::Ss58Codec;
    AccountId::from_ss58check(ss58).map_err(|e| format!("Invalid SS58 address '{}': {:?}", ss58, e))
}

/// Parse a hex-encoded Aura (sr25519) authority ID.
pub fn parse_aura_id(hex: &str) -> Result<AuraId, String> {
    let bytes = decode_hex(hex)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("Aura key '{}' must decode to exactly 32 bytes", hex))?;
    Ok(AuraId::unchecked_from(arr))
}

/// Parse a hex-encoded GRANDPA (ed25519) authority ID.
pub fn parse_grandpa_id(hex: &str) -> Result<GrandpaId, String> {
    let bytes = decode_hex(hex)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("GRANDPA key '{}' must decode to exactly 32 bytes", hex))?;
    Ok(GrandpaId::unchecked_from(arr))
}

/// Parse the full authorities JSON and return typed authority and account data.
///
/// Returns `(authorities, sudo_account, endowed_accounts_with_balances)`.
pub fn parse_authorities_config(
    json: &str,
) -> Result<
    (
        Vec<(AccountId, AccountId, AuraId, GrandpaId)>,
        AccountId,
        Vec<(AccountId, Balance)>,
    ),
    String,
> {
    let config: AuthoritiesConfig = serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse authorities JSON: {}", e))?;

    let authorities = config
        .authorities
        .iter()
        .map(|entry| {
            log::debug!(
                "Parsing authority '{}': stash={} aura={} grandpa={}",
                entry.name,
                entry.stash,
                entry.aura,
                entry.grandpa,
            );
            let stash = parse_account_id(&entry.stash)?;
            let controller = parse_account_id(&entry.controller)?;
            let aura = parse_aura_id(&entry.aura)?;
            let grandpa = parse_grandpa_id(&entry.grandpa)?;
            Ok((stash, controller, aura, grandpa))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let sudo = parse_account_id(&config.sudo)?;

    let endowed = config
        .endowed_accounts
        .iter()
        .map(|e| {
            let account = parse_account_id(&e.account)?;
            let balance: Balance = e.balance.parse().map_err(|_| {
                format!(
                    "Invalid balance '{}': must be a non-negative decimal integer",
                    e.balance
                )
            })?;
            Ok((account, balance))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok((authorities, sudo, endowed))
}

// ─── Chain specs ─────────────────────────────────────────────────────────────

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

/// Mainnet chain spec — reads authority config from an external JSON file.
///
/// The path to the JSON file is resolved from the `CLAWCHAIN_AUTHORITIES_FILE`
/// environment variable, falling back to `./authorities.json` in the working
/// directory.
///
/// # Errors
///
/// Returns an error if the authorities file cannot be read or parsed, or if
/// it contains no authorities.
pub fn mainnet_config() -> Result<ChainSpec, String> {
    let authorities_file = std::env::var("CLAWCHAIN_AUTHORITIES_FILE")
        .unwrap_or_else(|_| "./authorities.json".to_string());

    log::info!("Loading mainnet authorities from '{}'", authorities_file);

    let json = std::fs::read_to_string(&authorities_file).map_err(|e| {
        format!(
            "Failed to read authorities file '{}': {}. \
             Set CLAWCHAIN_AUTHORITIES_FILE or place authorities.json in the working directory.",
            authorities_file, e
        )
    })?;

    let (authorities, sudo, endowed_accounts) = parse_authorities_config(&json)?;

    if authorities.is_empty() {
        return Err(
            "Mainnet config requires at least one authority in the authorities file".to_string(),
        );
    }

    log::info!(
        "Loaded {} mainnet authorit{} from '{}'",
        authorities.len(),
        if authorities.len() == 1 { "y" } else { "ies" },
        authorities_file,
    );

    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Mainnet wasm not available".to_string())?,
        None,
    )
    .with_name("ClawChain Mainnet")
    .with_id("clawchain_mainnet")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(mainnet_genesis(authorities, sudo, endowed_accounts))
    .with_protocol_id("claw")
    .build())
}

// ─── Genesis builders ────────────────────────────────────────────────────────

/// Configure initial storage state for FRAME pallets (dev / testnet).
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
    let endowment: u128 =
        (VALIDATOR_ALLOCATION + TREASURY_ALLOCATION) / endowed_accounts.len() as u128;

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

/// Configure initial storage state for FRAME pallets (mainnet).
///
/// Unlike `testnet_genesis`, this takes explicit `(account, balance)` pairs
/// from the authorities file rather than computing an automatic endowment.
fn mainnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<(AccountId, Balance)>,
) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts,
        },
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(), // stash
                        x.0.clone(), // stash (bonded to itself in PoA)
                        session_keys(x.2.clone(), x.3.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        },
        "staking": {
            "validatorCount": initial_authorities.len() as u32,
            "minimumValidatorCount": 1u32,
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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known Substrate test vectors (Alice / Bob).
    const ALICE_SS58: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
    const BOB_SS58: &str = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty";

    // Alice's sr25519 public key (Aura).
    const ALICE_AURA_HEX: &str =
        "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    // Alice's ed25519 public key (GRANDPA).
    const ALICE_GRANDPA_HEX: &str =
        "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee";

    fn sample_json(
        stash: &str,
        controller: &str,
        aura_hex: &str,
        grandpa_hex: &str,
        sudo: &str,
        balance: &str,
    ) -> String {
        serde_json::json!({
            "authorities": [{
                "name": "validator-1",
                "stash": stash,
                "controller": controller,
                "aura": aura_hex,
                "grandpa": grandpa_hex,
            }],
            "sudo": sudo,
            "endowed_accounts": [{
                "account": stash,
                "balance": balance,
            }]
        })
        .to_string()
    }

    // ── decode_hex ────────────────────────────────────────────────────────────

    #[test]
    fn decode_hex_with_prefix() {
        assert_eq!(
            decode_hex("0xdeadbeef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
    }

    #[test]
    fn decode_hex_without_prefix() {
        assert_eq!(
            decode_hex("deadbeef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
    }

    #[test]
    fn decode_hex_empty_string() {
        assert_eq!(decode_hex("").unwrap(), Vec::<u8>::new());
        assert_eq!(decode_hex("0x").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn decode_hex_odd_length_returns_error() {
        let result = decode_hex("0xabc");
        assert!(result.is_err(), "Should fail on odd-length hex");
        assert!(result.unwrap_err().contains("odd length"));
    }

    #[test]
    fn decode_hex_invalid_char_returns_error() {
        let result = decode_hex("0xzz");
        assert!(result.is_err(), "Should fail on invalid hex char");
    }

    #[test]
    fn encode_hex_roundtrip() {
        let bytes = vec![0x00u8, 0xff, 0xab, 0x12];
        assert_eq!(encode_hex(&bytes), "00ffab12");
        assert_eq!(decode_hex(&encode_hex(&bytes)).unwrap(), bytes);
    }

    // ── parse_account_id ──────────────────────────────────────────────────────

    #[test]
    fn parse_account_id_valid_alice() {
        assert!(parse_account_id(ALICE_SS58).is_ok());
    }

    #[test]
    fn parse_account_id_invalid_returns_error() {
        let result = parse_account_id("not_a_valid_ss58_address");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("SS58"),
            "Error should mention SS58; got: {}",
            err
        );
    }

    // ── parse_aura_id ─────────────────────────────────────────────────────────

    #[test]
    fn parse_aura_id_valid() {
        assert!(parse_aura_id(ALICE_AURA_HEX).is_ok());
    }

    #[test]
    fn parse_aura_id_wrong_length_returns_error() {
        // 4 bytes — too short
        let result = parse_aura_id("0xdeadbeef");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("32 bytes"),
            "Error should mention 32 bytes; got: {}",
            err
        );
    }

    #[test]
    fn parse_aura_id_invalid_hex_returns_error() {
        assert!(parse_aura_id("0xinvalidhex!!").is_err());
    }

    // ── parse_grandpa_id ──────────────────────────────────────────────────────

    #[test]
    fn parse_grandpa_id_valid() {
        assert!(parse_grandpa_id(ALICE_GRANDPA_HEX).is_ok());
    }

    #[test]
    fn parse_grandpa_id_wrong_length_returns_error() {
        let result = parse_grandpa_id("0xdeadbeef");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("32 bytes"));
    }

    // ── parse_authorities_config ──────────────────────────────────────────────

    #[test]
    fn parse_authorities_config_valid() {
        let json = sample_json(
            ALICE_SS58,
            ALICE_SS58,
            ALICE_AURA_HEX,
            ALICE_GRANDPA_HEX,
            BOB_SS58,
            "500000000000000000000",
        );
        let result = parse_authorities_config(&json);
        assert!(
            result.is_ok(),
            "Should parse valid config: {:?}",
            result.err()
        );
        let (authorities, _sudo, endowed) = result.unwrap();
        assert_eq!(authorities.len(), 1);
        assert_eq!(endowed.len(), 1);
        assert_eq!(endowed[0].1, 500_000_000_000_000_000_000u128);
    }

    #[test]
    fn parse_authorities_config_multiple_validators() {
        let json = serde_json::json!({
            "authorities": [
                {
                    "name": "validator-1",
                    "stash": ALICE_SS58,
                    "controller": ALICE_SS58,
                    "aura": ALICE_AURA_HEX,
                    "grandpa": ALICE_GRANDPA_HEX,
                },
                {
                    "name": "validator-2",
                    "stash": BOB_SS58,
                    "controller": BOB_SS58,
                    "aura": ALICE_AURA_HEX,   // reuse hex for test purposes
                    "grandpa": ALICE_GRANDPA_HEX,
                }
            ],
            "sudo": ALICE_SS58,
            "endowed_accounts": []
        })
        .to_string();
        let (authorities, _, _) = parse_authorities_config(&json).unwrap();
        assert_eq!(authorities.len(), 2);
    }

    #[test]
    fn parse_authorities_config_empty_authorities_returns_ok() {
        // parse_authorities_config itself does NOT enforce non-empty; that's mainnet_config's job.
        let json = serde_json::json!({
            "authorities": [],
            "sudo": ALICE_SS58,
            "endowed_accounts": []
        })
        .to_string();
        let (authorities, _, _) = parse_authorities_config(&json).unwrap();
        assert!(authorities.is_empty());
    }

    #[test]
    fn parse_authorities_config_invalid_stash_ss58() {
        let json = sample_json(
            "not_valid_ss58",
            ALICE_SS58,
            ALICE_AURA_HEX,
            ALICE_GRANDPA_HEX,
            BOB_SS58,
            "1000",
        );
        let result = parse_authorities_config(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SS58"));
    }

    #[test]
    fn parse_authorities_config_invalid_controller_ss58() {
        let json = sample_json(
            ALICE_SS58,
            "bad_controller",
            ALICE_AURA_HEX,
            ALICE_GRANDPA_HEX,
            BOB_SS58,
            "1000",
        );
        assert!(parse_authorities_config(&json).is_err());
    }

    #[test]
    fn parse_authorities_config_invalid_aura_hex() {
        let json = sample_json(
            ALICE_SS58,
            ALICE_SS58,
            "0xinvalidhex!!",
            ALICE_GRANDPA_HEX,
            BOB_SS58,
            "1000",
        );
        assert!(parse_authorities_config(&json).is_err());
    }

    #[test]
    fn parse_authorities_config_aura_wrong_length() {
        let json = sample_json(
            ALICE_SS58,
            ALICE_SS58,
            "0xdeadbeef", // only 4 bytes
            ALICE_GRANDPA_HEX,
            BOB_SS58,
            "1000",
        );
        let result = parse_authorities_config(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("32 bytes"));
    }

    #[test]
    fn parse_authorities_config_invalid_grandpa_hex() {
        let json = sample_json(
            ALICE_SS58,
            ALICE_SS58,
            ALICE_AURA_HEX,
            "0xzzzzzzzz",
            BOB_SS58,
            "1000",
        );
        assert!(parse_authorities_config(&json).is_err());
    }

    #[test]
    fn parse_authorities_config_invalid_sudo_ss58() {
        let json = sample_json(
            ALICE_SS58,
            ALICE_SS58,
            ALICE_AURA_HEX,
            ALICE_GRANDPA_HEX,
            "bad_sudo",
            "1000",
        );
        let result = parse_authorities_config(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SS58"));
    }

    #[test]
    fn parse_authorities_config_invalid_balance() {
        let json = serde_json::json!({
            "authorities": [],
            "sudo": ALICE_SS58,
            "endowed_accounts": [{
                "account": ALICE_SS58,
                "balance": "not_a_number",
            }]
        })
        .to_string();
        let result = parse_authorities_config(&json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("balance"),
            "Error should mention balance; got: {}",
            err
        );
    }

    #[test]
    fn parse_authorities_config_malformed_json() {
        let result = parse_authorities_config("{invalid json}");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn parse_authorities_config_missing_field() {
        // Missing "sudo" field
        let result = parse_authorities_config(r#"{"authorities": [], "endowed_accounts": []}"#);
        assert!(result.is_err());
    }
}
