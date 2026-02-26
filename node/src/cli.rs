//! ClawChain CLI configuration.
//!
//! Defines the top-level [`Cli`] struct and all subcommands accepted by the
//! `clawchain-node` binary.

use sc_cli::RunCmd;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(
    name = "clawchain-node",
    about = "ClawChain: Agent-native Layer 1 blockchain node",
    version
)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    /// Run the node.
    #[clap(flatten)]
    pub run: RunCmd,
}

/// Available subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management CLI utilities.
    #[command(subcommand)]
    Key(sc_cli::KeySubcommand),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// Db meta columns information.
    ChainInfo(sc_cli::ChainInfoCmd),

    /// Generate a fresh Aura (sr25519) + GRANDPA (ed25519) keypair for use as
    /// a PoA validator.
    ///
    /// The output JSON contains the public keys in both 0x-hex and SS58
    /// formats together with the BIP39 secret phrase that can be inserted
    /// into the node's keystore via `clawchain-node key insert`.
    ///
    /// # Security
    ///
    /// The secret phrase gives **full control** over the validator signing
    /// keys.  Store it in an offline, encrypted vault and never commit it to
    /// version control.  Rotate keys according to the procedure described in
    /// `docs/authority-rotation.md`.
    GenerateKeys(GenerateKeysCmd),
}

/// Arguments for the `generate-keys` subcommand.
#[derive(Debug, clap::Args)]
pub struct GenerateKeysCmd {
    /// Write the generated keypair JSON to this file instead of stdout.
    ///
    /// The file is created (or overwritten) with mode 0o600 on Unix to
    /// limit read access to the current user.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

impl GenerateKeysCmd {
    /// Run the key generation command.
    ///
    /// Generates:
    /// - One **Aura** (sr25519) keypair for block authoring.
    /// - One **GRANDPA** (ed25519) keypair for block finality.
    ///
    /// Both keys are printed (or written to `--output`) as a JSON object with
    /// the following schema:
    ///
    /// ```json
    /// {
    ///   "aura": {
    ///     "type": "sr25519",
    ///     "public_key": "0x...",
    ///     "ss58_address": "5...",
    ///     "secret_phrase": "word1 word2 ... word12"
    ///   },
    ///   "grandpa": {
    ///     "type": "ed25519",
    ///     "public_key": "0x...",
    ///     "ss58_address": "5...",
    ///     "secret_phrase": "word1 word2 ... word12"
    ///   }
    /// }
    /// ```
    pub fn run(&self) -> sc_cli::Result<()> {
        use sp_core::{crypto::Ss58Codec, ed25519, sr25519, Pair};

        // Generate fresh, random keypairs with BIP39 phrases.
        let (aura_pair, aura_phrase, _) = sr25519::Pair::generate_with_phrase(None);
        let (grandpa_pair, grandpa_phrase, _) = ed25519::Pair::generate_with_phrase(None);

        let output = serde_json::json!({
            "aura": {
                "type": "sr25519",
                "public_key": format!("0x{}", encode_hex(aura_pair.public().as_ref())),
                "ss58_address": aura_pair.public().to_ss58check(),
                "secret_phrase": aura_phrase,
            },
            "grandpa": {
                "type": "ed25519",
                "public_key": format!("0x{}", encode_hex(grandpa_pair.public().as_ref())),
                "ss58_address": grandpa_pair.public().to_ss58check(),
                "secret_phrase": grandpa_phrase,
            },
        });

        let json_str = serde_json::to_string_pretty(&output)
            .map_err(|e| sc_cli::Error::Application(e.into()))?;

        match &self.output {
            Some(path) => {
                write_keys_file(path, &json_str)
                    .map_err(|e| sc_cli::Error::Application(e.into()))?;
                eprintln!("Keypair written to {}", path.display());
            }
            None => {
                println!("{}", json_str);
            }
        }

        Ok(())
    }
}

/// Write `content` to `path` with restricted permissions (0o600 on Unix).
fn write_keys_file(
    path: &std::path::Path,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Write as _;

    let mut file = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)?
        }
        #[cfg(not(unix))]
        {
            std::fs::File::create(path)?
        }
    };

    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Hex-encode bytes without a `0x` prefix.
fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::{crypto::Ss58Codec, ed25519, sr25519, Pair};

    // ── Key generation helpers ────────────────────────────────────────────────

    #[test]
    fn generate_aura_keypair_has_correct_length() {
        let (pair, _phrase, _) = sr25519::Pair::generate_with_phrase(None);
        assert_eq!(
            pair.public().as_ref().len(),
            32,
            "sr25519 public key must be 32 bytes"
        );
    }

    #[test]
    fn generate_grandpa_keypair_has_correct_length() {
        let (pair, _phrase, _) = ed25519::Pair::generate_with_phrase(None);
        assert_eq!(
            pair.public().as_ref().len(),
            32,
            "ed25519 public key must be 32 bytes"
        );
    }

    #[test]
    fn generated_phrase_is_non_empty() {
        let (_, aura_phrase, _) = sr25519::Pair::generate_with_phrase(None);
        let (_, grandpa_phrase, _) = ed25519::Pair::generate_with_phrase(None);
        assert!(!aura_phrase.is_empty(), "Aura phrase must not be empty");
        assert!(
            !grandpa_phrase.is_empty(),
            "GRANDPA phrase must not be empty"
        );
    }

    #[test]
    fn generated_phrase_is_twelve_words() {
        let (_, phrase, _) = sr25519::Pair::generate_with_phrase(None);
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(
            words.len(),
            12,
            "Generated phrase should be 12 words (BIP39)"
        );
    }

    #[test]
    fn two_generated_keypairs_are_distinct() {
        let (pair1, _phrase1, _) = sr25519::Pair::generate_with_phrase(None);
        let (pair2, _phrase2, _) = sr25519::Pair::generate_with_phrase(None);
        assert_ne!(
            pair1.public().as_ref(),
            pair2.public().as_ref(),
            "Two freshly generated keypairs must differ"
        );
    }

    #[test]
    fn ss58_address_has_correct_prefix() {
        // Substrate default SS58 version 42 starts with '5'
        let (pair, _, _) = sr25519::Pair::generate_with_phrase(None);
        let address = pair.public().to_ss58check();
        assert!(
            address.starts_with('5'),
            "Generic Substrate SS58 address should start with '5', got: {}",
            address
        );
    }

    #[test]
    fn public_key_hex_encoding_roundtrip() {
        let (pair, _, _) = sr25519::Pair::generate_with_phrase(None);
        let bytes = pair.public().as_ref();
        let hex = encode_hex(bytes);
        assert_eq!(hex.len(), 64, "32 bytes → 64 hex chars");
        // Decode back and compare
        let decoded: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(decoded, bytes);
    }

    // ── GenerateKeysCmd::run ─────────────────────────────────────────────────

    #[test]
    fn generate_keys_cmd_writes_to_file() {
        let tmp_dir = std::env::temp_dir();
        let output_path = tmp_dir.join(format!(
            "clawchain_test_keys_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));

        let cmd = GenerateKeysCmd {
            output: Some(output_path.clone()),
        };

        assert!(
            cmd.run().is_ok(),
            "generate-keys should succeed when writing to a file"
        );

        // File must exist and contain valid JSON
        let content =
            std::fs::read_to_string(&output_path).expect("Key file should have been created");
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("Key file should contain valid JSON");

        // Validate JSON structure
        assert!(
            json["aura"]["type"].as_str() == Some("sr25519"),
            "aura type should be sr25519"
        );
        assert!(
            json["grandpa"]["type"].as_str() == Some("ed25519"),
            "grandpa type should be ed25519"
        );
        assert!(
            json["aura"]["public_key"]
                .as_str()
                .unwrap_or("")
                .starts_with("0x"),
            "Aura public key should be 0x-prefixed hex"
        );
        assert!(
            json["grandpa"]["public_key"]
                .as_str()
                .unwrap_or("")
                .starts_with("0x"),
            "GRANDPA public key should be 0x-prefixed hex"
        );
        assert!(
            !json["aura"]["secret_phrase"]
                .as_str()
                .unwrap_or("")
                .is_empty(),
            "Aura secret phrase should not be empty"
        );
        assert!(
            !json["grandpa"]["secret_phrase"]
                .as_str()
                .unwrap_or("")
                .is_empty(),
            "GRANDPA secret phrase should not be empty"
        );

        // Validate public key lengths (32 bytes = 64 hex chars + "0x" prefix = 66)
        let aura_hex = json["aura"]["public_key"].as_str().unwrap();
        assert_eq!(
            aura_hex.len(),
            66,
            "Aura public key hex should be 66 chars (0x + 64)"
        );
        let grandpa_hex = json["grandpa"]["public_key"].as_str().unwrap();
        assert_eq!(
            grandpa_hex.len(),
            66,
            "GRANDPA public key hex should be 66 chars (0x + 64)"
        );

        // Validate SS58 addresses
        let aura_ss58 = json["aura"]["ss58_address"].as_str().unwrap();
        assert!(
            aura_ss58.starts_with('5'),
            "Aura SS58 should start with '5'"
        );
        let grandpa_ss58 = json["grandpa"]["ss58_address"].as_str().unwrap();
        assert!(
            grandpa_ss58.starts_with('5'),
            "GRANDPA SS58 should start with '5'"
        );

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn generate_keys_cmd_produces_unique_keys_each_call() {
        let tmp_dir = std::env::temp_dir();

        let path1 = tmp_dir.join("claw_keys_unique_test_1.json");
        let path2 = tmp_dir.join("claw_keys_unique_test_2.json");

        let _ = GenerateKeysCmd {
            output: Some(path1.clone()),
        }
        .run();
        let _ = GenerateKeysCmd {
            output: Some(path2.clone()),
        }
        .run();

        let json1: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path1).unwrap()).unwrap();
        let json2: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path2).unwrap()).unwrap();

        assert_ne!(
            json1["aura"]["public_key"], json2["aura"]["public_key"],
            "Two generate-keys runs should produce different Aura keys"
        );
        assert_ne!(
            json1["grandpa"]["public_key"], json2["grandpa"]["public_key"],
            "Two generate-keys runs should produce different GRANDPA keys"
        );

        let _ = std::fs::remove_file(&path1);
        let _ = std::fs::remove_file(&path2);
    }

    // ── encode_hex ────────────────────────────────────────────────────────────

    #[test]
    fn encode_hex_empty() {
        assert_eq!(encode_hex(&[]), "");
    }

    #[test]
    fn encode_hex_known_values() {
        assert_eq!(encode_hex(&[0x00, 0xff, 0xab]), "00ffab");
    }

    #[test]
    fn encode_hex_all_bytes() {
        let bytes: Vec<u8> = (0u8..=255).collect();
        let hex = encode_hex(&bytes);
        assert_eq!(hex.len(), 512);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
