use std::{
    collections::{HashMap, hash_map::Entry},
    fmt::{self, Display},
    fs::File,
    io::BufReader,
};

use chrono::NaiveDate;
use serde::Deserialize;
use thiserror::Error;

use crate::types::{ChainContracts, ChainDeployments, ContractName, ProtocolDeployments};

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
struct NetworkDeployments {
    #[serde(flatten)]
    deployments: HashMap<String, Deployment>,
}

#[derive(Debug, Deserialize, Clone)]
struct Deployment {
    version: DeploymentVersion,
    status: DeploymentStatus,
    contracts: Vec<Contract>,
}

#[derive(Debug, Deserialize, Clone)]
struct Contract {
    name: String,
    address: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
enum DeploymentStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "DEPRECATED")]
    Deprecated,
    /// The only SCRIPT known is Avalanche's 20250411-balancer-registry-initializer-v2
    #[serde(rename = "SCRIPT")]
    Script,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
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

pub fn parse(
    path_to_repo: &str,
) -> Result<(ProtocolDeployments, ProtocolDeployments), ParseError> {
    let path_to_folder = format!("{}/addresses", path_to_repo);

    let supported_networks = read_supported_networks(&path_to_folder)?;

    let mut v2_chains: ChainDeployments = HashMap::new();
    let mut v3_chains: ChainDeployments = HashMap::new();

    for (network, info) in supported_networks.networks {
        let deployments = read_deployments_from_network_file(&path_to_folder, &network)?;

        let active_v2_deployments = filter_active_deployments_by_version(
            &deployments,
            DeploymentVersion::V2,
        );
        let active_v3_deployments = filter_active_deployments_by_version(
            &deployments,
            DeploymentVersion::V3,
        );

        if !active_v2_deployments.is_empty() {
            let v2_contracts =
                process_contracts_with_latest_deployments(active_v2_deployments, info.chain_id)?;

            match v2_chains.entry(info.chain_id) {
                Entry::Occupied(_) => {
                    return Err(ParseError::ChainIdAlreadyExists {
                        chain_id: info.chain_id,
                    });
                }
                Entry::Vacant(entry) => {
                    entry.insert(v2_contracts);
                }
            }
        }

        if !active_v3_deployments.is_empty() {
            let v3_contracts =
                process_contracts_with_latest_deployments(active_v3_deployments, info.chain_id)?;

            match v3_chains.entry(info.chain_id) {
                Entry::Occupied(_) => {
                    return Err(ParseError::ChainIdAlreadyExists {
                        chain_id: info.chain_id,
                    });
                }
                Entry::Vacant(entry) => {
                    entry.insert(v3_contracts);
                }
            }
        }
    }

    Ok((
        ProtocolDeployments {
            protocol_name: "balancer-v2".to_string(),
            chains: v2_chains,
        },
        ProtocolDeployments {
            protocol_name: "balancer-v3".to_string(),
            chains: v3_chains,
        },
    ))
}

fn process_contracts_with_latest_deployments(
    active_deployments: HashMap<String, Deployment>,
    chain_id: u64,
) -> Result<ChainContracts, ParseError> {
    let mut contracts: ChainContracts = HashMap::new();
    let mut deployment_dates: HashMap<ContractName, NaiveDate> = HashMap::new();

    for (signature, deployment) in active_deployments {
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

fn filter_active_deployments_by_version(
    deployments: &NetworkDeployments,
    version: DeploymentVersion,
) -> HashMap<String, Deployment> {
    deployments
        .deployments
        .iter()
        .filter(|(_, deployment)| {
            deployment.version == version && deployment.status == DeploymentStatus::Active
        })
        .map(|(k, v)| (k.clone(), v.clone()))
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
) -> Result<NetworkDeployments, ParseError> {
    let file = File::open(format!("{}/{}.json", path_to_folder, network))?;
    let reader = BufReader::new(file);
    let deployments: NetworkDeployments = serde_json::from_reader(reader)?;

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
    use chrono::Datelike;

    #[test]
    fn test_parse_balancer() {
        let path = "source/balancer";
        let res = parse(path);
        assert!(res.is_ok());

        let (v2_deployments, v3_deployments) = res.unwrap();

        assert_eq!(v2_deployments.protocol_name, "balancer-v2");
        assert!(!v2_deployments.chains.is_empty());

        for (chain_id, contracts) in v2_deployments.chains {
            assert!(chain_id > 0);
            assert!(!contracts.is_empty());
        }

        assert_eq!(v3_deployments.protocol_name, "balancer-v3");
        assert!(!v3_deployments.chains.is_empty());

        for (chain_id, contracts) in v3_deployments.chains {
            assert!(chain_id > 0);
            assert!(!contracts.is_empty());
        }
    }

    #[test]
    fn test_parse_data_from_signature_valid() {
        let signature = "20250411-balancer-registry-initializer-v2".to_string();
        let result = parse_data_from_signature(signature, 1);

        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), 4);
        assert_eq!(date.day(), 11);
    }

    #[test]
    fn test_parse_data_from_signature_another_valid() {
        let signature = "20231225-some-deployment".to_string();
        let result = parse_data_from_signature(signature, 1);

        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 25);
    }

    #[test]
    fn test_parse_data_from_signature_invalid_no_date() {
        let signature = "invalid-signature".to_string();
        let result = parse_data_from_signature(signature.clone(), 1);

        assert!(result.is_err());
        match result {
            Err(ParseError::DateParseError(_)) => {}
            _ => panic!("Expected DateParseError"),
        }
    }

    #[test]
    fn test_parse_data_from_signature_empty() {
        let signature = "".to_string();
        let result = parse_data_from_signature(signature, 1);

        assert!(result.is_err());
        match result {
            Err(ParseError::DateParseError(_)) => {}
            _ => panic!("Expected DateParseError"),
        }
    }

    #[test]
    fn test_filter_active_deployments_all_active_v2() {
        let mut deployments_map = HashMap::new();
        deployments_map.insert(
            "20250101-deploy1".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![],
            },
        );
        deployments_map.insert(
            "20250102-deploy2".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![],
            },
        );

        let network_deployments = NetworkDeployments {
            deployments: deployments_map,
        };

        let result =
            filter_active_deployments_by_version(&network_deployments, DeploymentVersion::V2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_active_deployments_mixed() {
        let mut deployments_map = HashMap::new();
        deployments_map.insert(
            "20250101-deploy1".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![],
            },
        );
        deployments_map.insert(
            "20250102-deploy2".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Deprecated,
                contracts: vec![],
            },
        );
        deployments_map.insert(
            "20250103-deploy3".to_string(),
            Deployment {
                version: DeploymentVersion::V3,
                status: DeploymentStatus::Active,
                contracts: vec![],
            },
        );

        let network_deployments = NetworkDeployments {
            deployments: deployments_map,
        };

        let v2_result =
            filter_active_deployments_by_version(&network_deployments, DeploymentVersion::V2);
        assert_eq!(v2_result.len(), 1);
        assert!(v2_result.contains_key("20250101-deploy1"));

        let v3_result =
            filter_active_deployments_by_version(&network_deployments, DeploymentVersion::V3);
        assert_eq!(v3_result.len(), 1);
        assert!(v3_result.contains_key("20250103-deploy3"));
    }

    #[test]
    fn test_filter_active_deployments_empty() {
        let network_deployments = NetworkDeployments {
            deployments: HashMap::new(),
        };

        let result =
            filter_active_deployments_by_version(&network_deployments, DeploymentVersion::V2);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_process_contracts_single_deployment() {
        let mut deployments = HashMap::new();
        deployments.insert(
            "20250101-deploy".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![
                    Contract {
                        name: "Vault".to_string(),
                        address: "0x1234".to_string(),
                    },
                    Contract {
                        name: "Router".to_string(),
                        address: "0x5678".to_string(),
                    },
                ],
            },
        );

        let result = process_contracts_with_latest_deployments(deployments, 1);
        assert!(result.is_ok());

        let contracts = result.unwrap();
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts.get("Vault"), Some(&"0x1234".to_string()));
        assert_eq!(contracts.get("Router"), Some(&"0x5678".to_string()));
    }

    #[test]
    fn test_process_contracts_keeps_latest_deployment() {
        let mut deployments = HashMap::new();

        deployments.insert(
            "20240101-deploy1".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![Contract {
                    name: "Vault".to_string(),
                    address: "0xOLD".to_string(),
                }],
            },
        );

        deployments.insert(
            "20250101-deploy2".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![Contract {
                    name: "Vault".to_string(),
                    address: "0xNEW".to_string(),
                }],
            },
        );

        let result = process_contracts_with_latest_deployments(deployments, 1);
        assert!(result.is_ok());

        let contracts = result.unwrap();
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts.get("Vault"), Some(&"0xNEW".to_string()));
    }

    #[test]
    fn test_process_contracts_keeps_oldest_when_newer_comes_first() {
        let mut deployments = HashMap::new();

        deployments.insert(
            "20250101-deploy1".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![Contract {
                    name: "Vault".to_string(),
                    address: "0xNEW".to_string(),
                }],
            },
        );

        deployments.insert(
            "20240101-deploy2".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![Contract {
                    name: "Vault".to_string(),
                    address: "0xOLD".to_string(),
                }],
            },
        );

        let result = process_contracts_with_latest_deployments(deployments, 1);
        assert!(result.is_ok());

        let contracts = result.unwrap();
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts.get("Vault"), Some(&"0xNEW".to_string()));
    }

    #[test]
    fn test_process_contracts_multiple_contracts_different_dates() {
        let mut deployments = HashMap::new();

        deployments.insert(
            "20240101-deploy1".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![
                    Contract {
                        name: "Vault".to_string(),
                        address: "0xVaultOld".to_string(),
                    },
                    Contract {
                        name: "Router".to_string(),
                        address: "0xRouterOld".to_string(),
                    },
                ],
            },
        );

        deployments.insert(
            "20250101-deploy2".to_string(),
            Deployment {
                version: DeploymentVersion::V2,
                status: DeploymentStatus::Active,
                contracts: vec![Contract {
                    name: "Vault".to_string(),
                    address: "0xVaultNew".to_string(),
                }],
            },
        );

        let result = process_contracts_with_latest_deployments(deployments, 1);
        assert!(result.is_ok());

        let contracts = result.unwrap();
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts.get("Vault"), Some(&"0xVaultNew".to_string()));
        assert_eq!(contracts.get("Router"), Some(&"0xRouterOld".to_string()));
    }

    #[test]
    fn test_process_contracts_empty_deployments() {
        let deployments = HashMap::new();
        let result = process_contracts_with_latest_deployments(deployments, 1);

        assert!(result.is_ok());
        let contracts = result.unwrap();
        assert_eq!(contracts.len(), 0);
    }
}
