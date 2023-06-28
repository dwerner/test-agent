use std::{
    fmt::{Display, Formatter},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use casper_node::{utils::External, MainReactorConfig};

use casper_types::{
    AccountConfig, AccountsConfig, Chainspec, ChainspecRawBytes, DelegatorConfig, ValidatorConfig,
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

use casper_types::{Motes, ProtocolVersion, PublicKey, SecretKey, U512};
use const_format::concatcp;
use structopt::StructOpt;

use crate::{common, compile::BuildArtifacts};

const DEFAULT_ASSETS_PATH: &str = concatcp!(common::BUILD_DIR, "/assets");
const DEFAULT_CHAINSPEC_SRC_PATH: &str =
    concatcp!(common::BUILD_DIR, "/casper-node/resources/production/");

#[derive(StructOpt, Debug)]
pub struct GenerateNetworkAssets {
    /// Name of the network
    network_name: String,

    /// Version of the staged network
    #[structopt(short, long, parse(try_from_str = Version::from_str), default_value = "1.0.0")]
    version: Version,

    /// Path to the assets directory
    #[structopt(short, default_value = DEFAULT_ASSETS_PATH)]
    assets_path: PathBuf,

    /// Path to the chainspec source directory
    #[structopt(short, default_value = DEFAULT_CHAINSPEC_SRC_PATH)]
    chainspec_src_path: PathBuf,

    /// Path to the chainspec source directory
    #[structopt(subcommand)]
    source: Params,

    /// Overwrite existing files
    #[structopt(short, long)]
    overwrite: bool,
}

#[derive(StructOpt, Debug)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let major = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing major version"))?
            .parse::<u32>()?;
        let minor = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing minor version"))?
            .parse::<u32>()?;
        let patch = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing patch version"))?
            .parse::<u32>()?;
        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(StructOpt, Debug)]
pub enum Params {
    /// Generate a chainspec from scratch, specifying all parameters
    Generate {
        /// Number of validators to generate
        validator_count: u32,
        /// Balance for each validator
        validator_balance: u64,
        /// Bonded amount for each validator
        validator_bonded_amount: u64,
        /// Number of delegators to generate
        delegator_count: u32,
        /// Balance for each delegator
        delegator_balance: u64,
        /// Delegated amount for each delegator
        delegated_amount: u64,
    },
    /// Use the default values
    Default,
    /// Use the default values, but override the number of validators
    Validators { count: u32 },
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

/// Generate assets for a given network. Generates the files within the assets directory.
/// This includes:
/// - accounts.toml
/// - chainspec.toml
/// - config.toml
/// - validator keys
/// - delegator keys
pub fn generate_network_config_assets(
    GenerateNetworkAssets {
        network_name,
        assets_path,
        chainspec_src_path,
        source,
        overwrite,
        version,
    }: GenerateNetworkAssets,
) -> Result<BuildArtifacts, anyhow::Error> {
    println!(
        "Generating network assets for network '{}' version '{}'...",
        network_name, version,
    );
    let network_dir = assets_path.join(&network_name).join(format!(
        "{}_{}_{}",
        version.major, version.minor, version.patch
    ));

    if network_dir.exists() {
        if overwrite {
            fs::remove_dir_all(&network_dir)?;
        } else {
            return Err(anyhow::anyhow!(
                "network dir already exists at {}",
                network_dir.display()
            ));
        }
    }

    fs::create_dir_all(&network_dir)?;

    // shared directory containing files that are shared between nodes
    let network_shared_dir = network_dir.join("shared");
    fs::create_dir_all(&network_shared_dir)?;

    create_accounts_toml_from_params(source, &network_shared_dir)?;
    create_chainspec_from_src(
        &chainspec_src_path,
        &network_name,
        &network_shared_dir,
        version,
    )?;
    create_config_from_defaults(&network_shared_dir)?;

    Ok(BuildArtifacts {
        path: network_shared_dir,
        files: vec![
            "accounts.toml".to_string(),
            "chainspec.toml".to_string(),
            "config.toml".to_string(),
        ],
    })
}

/// Create accounts.toml from the given parameters
fn create_accounts_toml_from_params(
    source: Params,
    network_shared_dir: &Path,
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
                network_shared_dir,
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
                network_shared_dir,
                validator.public_key.clone(),
                delegator_balance,
                delegated_amount,
            )?;
            delegators.push(delegator);
        }
        let accounts_config = AccountsConfig::new(accounts, delegators);

        // Write accounts.toml
        let accounts = toml::to_string_pretty(&accounts_config)?;
        let mut writer = BufWriter::new(File::create(network_shared_dir.join(ACCOUNTS_TOML))?);
        writer.write_all(accounts.as_bytes())?;
        writer.flush()?;
    } else {
        unreachable!()
    }
    Ok(())
}

fn create_config_from_defaults(network_shared_dir: &Path) -> Result<(), anyhow::Error> {
    let mut config = MainReactorConfig::default();
    let path = Path::new(SECRET_KEY_PEM);
    config.consensus.secret_key_path = External::Path(path.to_path_buf());
    let config = toml::to_string_pretty(&config)?;
    let mut writer = BufWriter::new(File::create(network_shared_dir.join(CONFIG_TOML))?);
    writer.write_all(config.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn create_chainspec_from_src(
    chainspec_src_path: &Path,
    network_name: &str,
    network_shared_dir: &Path,
    version: Version,
) -> Result<(), anyhow::Error> {
    use casper_node::utils::Loadable;
    let (mut chainspec, _chainspec_raw_bytes) =
        <(Chainspec, ChainspecRawBytes)>::from_path(chainspec_src_path)?;
    chainspec.network_config.name = network_name.to_owned();
    chainspec.protocol_config.version =
        ProtocolVersion::from_parts(version.major, version.minor, version.patch);
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
    let mut writer = BufWriter::new(File::create(network_shared_dir.join(CHAINSPEC_TOML))?);
    writer.write_all(chainspec.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// Create a validator account and write public and private keys to disk.
fn create_validator_account(
    id: u32,
    network_asset_dir: &Path,
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
    network_asset_dir: &Path,
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
        return Err(anyhow::anyhow!("unsupported algorithm {}", algorithm));
    };
    let public_key = PublicKey::from(&secret_key);
    let secret_key_path = output_dir.join(SECRET_KEY_PEM);
    secret_key.to_file(secret_key_path)?;

    let public_key_path = output_dir.join(PUBLIC_KEY_PEM);
    public_key.to_file(public_key_path)?;

    Ok((public_key, secret_key))
}
