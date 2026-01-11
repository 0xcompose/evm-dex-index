use std::{
    collections::{HashMap, hash_map::Entry},
    fmt::{self, Display},
    fs::File,
    io::BufReader,
};

use chrono::NaiveDate;
use serde::Deserialize;
use thiserror::Error;

use crate::parser::ProtocolDeployment;

#[derive(Debug, Deserialize)]
struct SupportedNetworks {
    #[serde(flatten)]
    pub networks: HashMap<String, NetworkInfo>,
}

#[derive(Debug, Deserialize)]
struct NetworkInfo {
    #[serde(rename = "chainId")]
    chain_id: u64,
}

#[derive(Debug, Deserialize)]
struct ChainDeployments {
    #[serde(flatten)]
    deployments: HashMap<String, Deployment>,
}

#[derive(Debug, Deserialize)]
struct Deployment {
    version: DeploymentVersion,
    status: DeploymentStatus,
    contracts: Vec<Contract>,
}

#[derive(Debug, Deserialize)]
struct Contract {
    name: String,
    address: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
enum DeploymentStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "DEPRECATED")]
    Deprecated,
    /// The only SCRIPT known is Avalanche's 20250411-balancer-registry-initializer-v2
    #[serde(rename = "SCRIPT")]
    Script,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
enum DeploymentVersion {
    #[serde(rename = "v2")]
    V2,
    #[serde(rename = "v3")]
    V3,
}

#[derive(Debug, Error)]
pub enum ParseError {
    ChainIdAlreadyExists { chain_id: u64 },

    NoDateInSignature { chain_id: u64, signature: String },

    IoError(#[from] std::io::Error),

    SerdeError(#[from] serde_json::Error),

    DateParseError(#[from] chrono::ParseError),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub fn parse(path_to_repo: &str) -> Result<HashMap<u64, ProtocolDeployment>, ParseError> {
    let path_to_folder = format!("{}/addresses", path_to_repo);

    let supported_networks = read_supported_networks(&path_to_folder)?;

    let mut protocol_deployments: HashMap<u64, ProtocolDeployment> = HashMap::new();

    for (network, info) in supported_networks.networks {
        let deployments = read_deployments_from_network_file(&path_to_folder, &network)?;
        let active_v2_deployments = filter_active_v2_deployments(deployments);

        let contracts =
            process_contracts_with_latest_deployments(active_v2_deployments, info.chain_id)?;
        let protocol_deployment = ProtocolDeployment::new("balancer-v2", info.chain_id, contracts);

        match protocol_deployments.entry(info.chain_id) {
            Entry::Occupied(_) => {
                return Err(ParseError::ChainIdAlreadyExists {
                    chain_id: info.chain_id,
                });
            }
            Entry::Vacant(entry) => {
                entry.insert(protocol_deployment);
            }
        }
    }

    Ok(protocol_deployments)
}

fn process_contracts_with_latest_deployments(
    active_v2_deployments: HashMap<String, Deployment>,
    chain_id: u64,
) -> Result<HashMap<String, String>, ParseError> {
    let mut contracts = HashMap::new();
    let mut deployment_dates: HashMap<String, NaiveDate> = HashMap::new();

    for (signature, deployment) in active_v2_deployments {
        let date = parse_data_from_signature(signature, chain_id)?;

        for contract in deployment.contracts {
            let should_update = deployment_dates
                .get(&contract.name)
                .map_or(true, |existing_date| date >= *existing_date);

            if should_update {
                contracts.insert(contract.name.clone(), contract.address);
                deployment_dates.insert(contract.name, date);
            }
        }
    }

    Ok(contracts)
}

fn filter_active_v2_deployments(deployments: ChainDeployments) -> HashMap<String, Deployment> {
    deployments
        .deployments
        .into_iter()
        .filter(|(_, deployment)| {
            deployment.version == DeploymentVersion::V2
                && deployment.status == DeploymentStatus::Active
        })
        .collect::<HashMap<String, Deployment>>()
}

fn read_supported_networks(path_to_folder: &str) -> Result<SupportedNetworks, ParseError> {
    let supported_networks = File::open(format!("{}/.supported-networks.json", path_to_folder))?;
    let reader = BufReader::new(supported_networks);
    let supported_networks: SupportedNetworks = serde_json::from_reader(reader)?;

    Ok(supported_networks)
}

fn read_deployments_from_network_file(
    path_to_folder: &str,
    network: &str,
) -> Result<ChainDeployments, ParseError> {
    let file = File::open(format!("{}/{}.json", path_to_folder, network))?;
    let reader = BufReader::new(file);
    let deployments: ChainDeployments = serde_json::from_reader(reader)?;

    Ok(deployments)
}

fn parse_data_from_signature(signature: String, chain_id: u64) -> Result<NaiveDate, ParseError> {
    // yyyymmdd format, example: 20250411

    let date_str = signature
        .split('-')
        .nth(0)
        .ok_or(ParseError::NoDateInSignature {
            chain_id,
            signature: signature.to_owned(),
        })?;

    let date = NaiveDate::parse_from_str(date_str, "%Y%m%d")?;

    Ok(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_balancer_v2() {
        let path = "source/balancer";
        let res = parse(path);
        dbg!(&res);
        assert!(res.is_ok());
    }
}
