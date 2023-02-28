use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use casper_node::{
    types::{
        chainspec::{
            AccountConfig, AccountsConfig, ChainspecRawBytes, DelegatorConfig, ValidatorConfig,
        },
        Chainspec,
    },
    utils::External,
    MainReactorConfig,
};

const ACCOUNTS_TOML: &str = "accounts.toml";
const CHAINSPEC_TOML: &str = "chainspec.toml";
const CONFIG_TOML: &str = "config.toml";
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
const DEFAULT_CHAINSPEC_SRC_PATH: &str =
    concatcp!(common::BUILD_DIR, "/casper-node/resources/production/");

#[derive(StructOpt, Debug)]
pub struct GenerateNetworkAssets {
    network_name: String,

    #[structopt(short, default_value = DEFAULT_ASSETS_PATH)]
    assets_path: PathBuf,

    #[structopt(short, default_value = DEFAULT_CHAINSPEC_SRC_PATH)]
    chainspec_src_path: PathBuf,

    #[structopt(subcommand)]
    source: Params,
}

#[derive(StructOpt, Debug)]
pub enum Params {
    Generate {
        validator_count: u32,
        validator_balance: u64,
        validator_bonded_amount: u64,
        delegator_count: u32,
        delegator_balance: u64,
        delegated_amount: u64,
    },
    Default,
    Validators {
        count: u32,
    },
}

impl Params {
    fn validator_count(validator_count: u32) -> Self {
        Params::Generate {
            validator_count,
            validator_balance: 100_000_000_000 * 1_000_000,
            validator_bonded_amount: 100_000_000_000 * 1_000_000,
            delegator_count: 10,
            delegator_balance: 1_000_000 * 1_000_000,
            delegated_amount: 500_000 * 1_000_000,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::validator_count(10)
    }
}

pub fn generate_network_assets(
    GenerateNetworkAssets {
        network_name,
        assets_path,
        chainspec_src_path,
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
    create_accounts_toml_from_params(source, &network_dir)?;
    create_chainspec_from_src(&chainspec_src_path, &network_name, &network_dir)?;
    create_config_from_defaults(&network_dir)?;

    Ok(())
}

fn create_accounts_toml_from_params(
    source: Params,
    network_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    if let Params::Generate {
        validator_count,
        validator_balance,
        validator_bonded_amount,
        delegator_count,
        delegator_balance,
        delegated_amount,
    } = match source {
        params @ Params::Generate { .. } => params,
        Params::Default => Params::default(),
        Params::Validators { count } => Params::validator_count(count),
    } {
        let mut accounts = vec![];
        for v in 0..validator_count {
            let validator = create_validator_account(
                v,
                network_dir,
                validator_balance,
                validator_bonded_amount,
            )?;
            accounts.push(validator);
        }
        let mut delegators = vec![];
        let mut validator_cycle_iter = accounts.iter().cycle();
        for d in 0..delegator_count as usize {
            let validator = validator_cycle_iter
                .next()
                .expect("None from an infinite loop?");
            let delegator = create_delegator_account(
                d as u32,
                network_dir,
                validator.public_key.clone(),
                delegator_balance,
                delegated_amount,
            )?;
            delegators.push(delegator);
        }
        let accounts_config = AccountsConfig::new(accounts, delegators);

        // Write accounts.toml
        let accounts = toml::to_string_pretty(&accounts_config)?;
        let mut writer = BufWriter::new(File::create(&network_dir.join(ACCOUNTS_TOML))?);
        writer.write_all(accounts.as_bytes())?;
        writer.flush()?;
    } else {
        unreachable!()
    }
    Ok(())
}

fn create_config_from_defaults(network_dir: &PathBuf) -> Result<(), anyhow::Error> {
    let mut config = MainReactorConfig::default();
    let path = Path::new(SECRET_KEY_PEM);
    config.consensus.secret_key_path = External::Path(path.to_path_buf());
    let config = toml::to_string_pretty(&config)?;
    let mut writer = BufWriter::new(File::create(&network_dir.join(CONFIG_TOML))?);
    writer.write_all(config.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn create_chainspec_from_src(
    chainspec_src_path: &PathBuf,
    network_name: &str,
    target_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    use casper_node::utils::Loadable;
    let (mut chainspec, _chainspec_raw_bytes) =
        <(Chainspec, ChainspecRawBytes)>::from_path(&chainspec_src_path)?;
    chainspec.network_config.name = network_name.to_owned();
    let chainspec = toml::to_string_pretty(&chainspec)?;

    // The node expects an accounts.toml and a chainspec.toml, but the above will add defaults from the node.
    // The node also can't represent this section being 'undefined' or removed, so the hacky workaround here
    // is to eliminate the network.accounts_config section manually by removing it from the toml value.
    let mut chainspec: toml::Value = toml::from_str(&chainspec)?;
    if let Some(network_section) = chainspec
        .get_mut("network")
        .iter_mut()
        .flat_map(|elem| elem.as_table_mut())
        .next()
    {
        network_section
            .remove("accounts_config")
            .expect("should have removed accounts_config section");
    }
    let chainspec = toml::to_string_pretty(&chainspec)?;
    let mut writer = BufWriter::new(File::create(&target_dir.join(CHAINSPEC_TOML))?);
    writer.write_all(chainspec.as_bytes())?;
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

/// Create a delegator account and write public and private keys to disk.
fn create_delegator_account(
    id: u32,
    network_asset_dir: &PathBuf,
    validator_public_key: PublicKey,
    balance: impl Into<U512>,
    delegated_amount: impl Into<U512>,
) -> Result<DelegatorConfig, anyhow::Error> {
    let path = network_asset_dir.join(format!("delegator-{id}"));
    let (delegator_public_key, _secret) =
        generate_keys(&path, if id % 2 == 0 { ED25519 } else { SECP256K1 })?;
    Ok(DelegatorConfig::new(
        validator_public_key,
        delegator_public_key,
        Motes::new(balance.into()),
        Motes::new(delegated_amount.into()),
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
