use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use casper_node::{
    logging::LoggingConfig,
    types::{
        chainspec::{AccountConfig, AccountsConfig, ChainspecRawBytes, ValidatorConfig},
        Chainspec,
    },
    MainReactorConfig,
};

/// Default filename for the PEM-encoded secret key file.
const SECRET_KEY_PEM: &str = "secret_key.pem";
/// Default filename for the PEM-encoded public key file.
const PUBLIC_KEY_PEM: &str = "public_key.pem";

/// Name of Ed25519 algorithm.
const ED25519: &str = "Ed25519";
/// Name of secp256k1 algorithm.
const SECP256K1: &str = "secp256k1";

use casper_types::{Motes, PublicKey, SecretKey, TimeDiff, Timestamp, U512};
use const_format::concatcp;
use duct::cmd;
use structopt::StructOpt;
use toml::Value;

use crate::common;

const DEFAULT_ASSETS_PATH: &str = concatcp!(common::BUILD_DIR, "/assets");

#[derive(StructOpt, Debug)]
pub struct GenerateNetworkAssets {
    network_name: String,

    #[structopt(short, default_value = DEFAULT_ASSETS_PATH)]
    assets_path: PathBuf,

    #[structopt(subcommand)]
    source: AssetSource,
}

#[derive(StructOpt, Debug)]
pub enum AssetSource {
    Generate {
        validator_count: u32,
        delegator_count: u32,
    },
    Template {
        template_src_path: PathBuf,
    },
}

pub fn generate_network_assets(
    GenerateNetworkAssets {
        network_name,
        assets_path,
        source,
    }: GenerateNetworkAssets,
) -> Result<(), anyhow::Error> {
    println!("generating network assets for {network_name}");
    let network_dir = assets_path.join(&network_name);
    if network_dir.exists() {
        return Err(anyhow::anyhow!(
            "network dir already exists at {}",
            network_dir.display()
        ));
    }

    fs::create_dir_all(&network_dir)?;

    match source {
        AssetSource::Template { template_src_path } => {
            generate_assets_from_local_template(&template_src_path, &network_dir, 60)
        }
        AssetSource::Generate {
            validator_count,
            delegator_count: _,
        } => generate_assets_with_counts(validator_count, &network_dir),
    }
}

fn generate_assets_from_local_template(
    template_src_path: &PathBuf,
    assets_target_path: &PathBuf,
    genesis_delay_secs: u32,
) -> Result<(), anyhow::Error> {
    let accounts_toml_path = template_src_path.join("accounts.toml");
    let config_toml_path = template_src_path.join("config.toml");
    let chainspec_template_path = template_src_path.join("chainspec.toml.in");

    let target_chainspec_path = assets_target_path.join("chainspec.toml");

    {
        use casper_node::utils::Loadable;
        // write chainspec
        let chainspec_template = fs::read_to_string(chainspec_template_path)?;
        let timestamp = Timestamp::now() + TimeDiff::from_seconds(genesis_delay_secs);

        let mut values = HashMap::new();
        values.insert("TIMESTAMP".into(), timestamp.to_string());
        let chainspec_str = envsubst::substitute(&chainspec_template, &values)?;
        let mut writer = BufWriter::new(File::create(&target_chainspec_path)?);
        writer.write_all(chainspec_str.as_bytes())?;
        writer.flush()?;
        let (chainspec, chainspec_raw_bytes) =
            <(Chainspec, ChainspecRawBytes)>::from_path(&assets_target_path)?;

        if !chainspec.is_valid() {
            return Err(anyhow::anyhow!("generated chainspec is invalid"));
        }
    }

    // write config.toml
    cmd!(
        "cp",
        &config_toml_path,
        assets_target_path.join("config.toml")
    )
    .run()?;

    // write accounts.toml
    cmd!(
        "cp",
        &accounts_toml_path,
        assets_target_path.join("accounts.toml")
    )
    .run()?;

    let config_str = fs::read_to_string(&config_toml_path)?;
    let config_table: Value = toml::from_str(&config_str)?;
    let main_config: MainReactorConfig = config_table.try_into()?;

    Ok(())
}

fn generate_assets_with_counts(
    validator_count: u32,
    network_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    let faucet = ();

    let mut accounts = vec![];
    for v in 0..validator_count {
        let validator = create_validator_account(v, network_dir, 12345, 12345)?;
        accounts.push(validator);
    }
    let accounts_config = AccountsConfig::new(accounts, vec![]);

    let accounts = dbg!(toml::to_string_pretty(&accounts_config)?);
    let mut writer = BufWriter::new(File::create(&network_dir.join("accounts.toml"))?);
    writer.write_all(accounts.as_bytes())?;
    writer.flush()?;

    Ok(())
}

/// Create a validator account and write public and private keys to disk.
fn create_validator_account(
    id: u32,
    network_asset_dir: &PathBuf,
    balance: impl Into<U512>,
    bonded_amount: impl Into<U512>,
) -> Result<AccountConfig, anyhow::Error> {
    let path = network_asset_dir.join(format!("validator-{id}"));
    let (pubkey, _secret) = generate_keys(&path, if id % 2 == 0 { ED25519 } else { SECP256K1 })?;
    let config = Some(ValidatorConfig::new(Motes::new(bonded_amount.into()), 0));
    Ok(AccountConfig::new(
        pubkey,
        Motes::new(balance.into()),
        config,
    ))
}

/// Generate a PublicKey+SecretKey pair(and the hex form), save them to assets and return their source objects.
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
