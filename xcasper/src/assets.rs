use std::{
    fs,
    path::{Path, PathBuf},
};

use casper_node::types::chainspec::{AccountConfig, AccountsConfig, ValidatorConfig};

/// Default filename for the PEM-encoded secret key file.
const SECRET_KEY_PEM: &str = "secret_key.pem";
/// Default filename for the PEM-encoded public key file.
const PUBLIC_KEY_PEM: &str = "public_key.pem";

/// Name of Ed25519 algorithm.
const ED25519: &str = "Ed25519";
/// Name of secp256k1 algorithm.
const SECP256K1: &str = "secp256k1";

use casper_types::{Motes, PublicKey, SecretKey, U512};
use const_format::concatcp;
use structopt::StructOpt;

use crate::common;

const DEFAULT_ASSETS_PATH: &str = concatcp!(common::BUILD_DIR, "/assets");

#[derive(StructOpt, Debug)]
pub struct GenerateNetworkAssets {
    network_name: String,
    #[structopt(short, default_value = DEFAULT_ASSETS_PATH)]
    assets_path: PathBuf,
    validator_count: u32,
    delegator_count: u32,
}

pub fn generate_network_assets(
    GenerateNetworkAssets {
        network_name,
        assets_path,
        validator_count,
        delegator_count: _,
    }: GenerateNetworkAssets,
) -> Result<(), anyhow::Error> {
    println!("generating network assets for {network_name}");
    let network_dir = assets_path.join(&network_name);
    fs::create_dir_all(&network_dir)?;

    {
        let mut accounts = vec![];

        // generate public and private keys
        for v in 0..validator_count {
            accounts.push(create_validator_assets_and_return_account(
                v,
                &network_dir,
                U512::from(12345),
                U512::from(12345),
            )?);
        }
        // create accounts.toml
        let accounts_config = AccountsConfig::new(accounts, vec![]);

        // create chainspec.toml
        // create config.toml
        // create global_state.toml (if needed)
    }
    Ok(())
}

fn create_validator_assets_and_return_account(
    id: u32,
    network_dir: &PathBuf,
    balance: U512,
    bonded_amount: U512,
) -> Result<AccountConfig, anyhow::Error> {
    let path = network_dir.join(format!("validator-{id}"));
    let (pubkey, _secret) = generate_keys(&path, "ed25519")?;
    let config = Some(ValidatorConfig::new(Motes::new(bonded_amount), 0));
    Ok(AccountConfig::new(pubkey, Motes::new(balance), config))
}

/// Generate a PublicKey+SecretKey pair, save them to assets and return their source objects.
fn generate_keys(
    output_dir: &PathBuf,
    algorithm: &str,
) -> Result<(PublicKey, SecretKey), anyhow::Error> {
    fs::create_dir_all(output_dir)?;
    let output_dir = Path::new(output_dir).canonicalize()?;
    let secret_key = if algorithm.eq_ignore_ascii_case(ED25519) {
        SecretKey::generate_ed25519()?
    } else if algorithm.eq_ignore_ascii_case(SECP256K1) {
        SecretKey::generate_secp256k1()?
    } else {
        return Err(anyhow::anyhow!("unsupported algorithm {}", algorithm).into());
    };
    let public_key = PublicKey::from(&secret_key);
    let secret_key_path = output_dir.join(SECRET_KEY_PEM);
    secret_key.to_file(&secret_key_path)?;
    let public_key_path = output_dir.join(PUBLIC_KEY_PEM);
    public_key.to_file(&public_key_path)?;

    Ok((public_key, secret_key))
}
