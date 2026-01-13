use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
};

use serde::Deserialize;
use thiserror::Error;
use tracing::debug;

use crate::types::{ChainContracts, ChainDeployments, ProtocolDeployments};

#[derive(Debug, Deserialize)]
struct ChainDeployment {
    #[serde(rename = "chainId")]
    chain_id: String,
    latest: HashMap<String, ContractDeployment>,
}

#[derive(Debug, Deserialize)]
struct ContractDeployment {
    address: String,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Missing contracts for protocol '{protocol_name}': {contracts:?}")]
    MissingContracts {
        protocol_name: String,
        contracts: Vec<String>,
    },

    #[error("Contract '{contract_name}' is defined in multiple protocols: {protocols:?}")]
    DuplicateContracts {
        contract_name: String,
        protocols: Vec<String>,
    },
}

struct ProtocolConfig {
    protocol_name: &'static str,
    contracts: &'static [&'static str],
}

const PROTOCOL_CONFIGS: &[ProtocolConfig] = &[
    ProtocolConfig {
        protocol_name: "uniswap-v2",
        contracts: &["UniswapV2Factory", "UniswapV2Router02"],
    },
    ProtocolConfig {
        protocol_name: "uniswap-v3",
        contracts: &[
            "UniswapV3Factory",
            "SwapRouter",
            "SwapRouter02",
            "NonfungiblePositionManager",
            "NonfungibleTokenPositionDescriptor",
            "NFTDescriptor",
            "Quoter",
            "QuoterV2",
            "TickLens",
            "V3Migrator",
        ],
    },
    ProtocolConfig {
        protocol_name: "uniswap-v4",
        contracts: &[
            "PoolManager",
            "PositionManager",
            "StateView",
            "PositionDescriptor",
            "V4Quoter",
            "WETHHook",
            "WstETHHook",
            "WstETHRoutingHook",
        ],
    },
    ProtocolConfig {
        protocol_name: "universal-router",
        contracts: &["UniversalRouter"],
    },
    ProtocolConfig {
        protocol_name: "permit2",
        contracts: &["Permit2"],
    },
];

fn validate_no_duplicate_contracts() -> Result<(), ParseError> {
    let mut contract_to_protocols: HashMap<&str, Vec<&str>> = HashMap::new();

    for config in PROTOCOL_CONFIGS {
        for &contract in config.contracts {
            contract_to_protocols
                .entry(contract)
                .or_insert_with(Vec::new)
                .push(config.protocol_name);
        }
    }

    for (contract_name, protocols) in contract_to_protocols {
        if protocols.len() > 1 {
            return Err(ParseError::DuplicateContracts {
                contract_name: contract_name.to_string(),
                protocols: protocols.iter().map(|s| s.to_string()).collect(),
            });
        }
    }

    Ok(())
}

pub fn parse(path_to_deployments: &str) -> Result<Vec<ProtocolDeployments>, ParseError> {
    validate_no_duplicate_contracts()?;

    let mut protocol_chains: HashMap<&str, ChainDeployments> = HashMap::new();
    let mut found_contracts: HashMap<&str, HashSet<&str>> = HashMap::new();

    for config in PROTOCOL_CONFIGS {
        protocol_chains.insert(config.protocol_name, HashMap::new());
        found_contracts.insert(config.protocol_name, HashSet::new());
    }

    let entries = std::fs::read_dir(path_to_deployments)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let deployment: ChainDeployment = serde_json::from_reader(reader)?;

        let chain_id: u64 = deployment.chain_id.parse().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid chain_id: {}", deployment.chain_id),
            )
        })?;

        let mut chain_protocol_contracts: HashMap<&str, ChainContracts> = HashMap::new();
        for config in PROTOCOL_CONFIGS {
            chain_protocol_contracts.insert(config.protocol_name, ChainContracts::new());
        }

        for (name, contract) in deployment.latest {
            let mut matched = false;
            for config in PROTOCOL_CONFIGS {
                if let Some(&contract_name) = config.contracts.iter().find(|&&c| c == name.as_str())
                {
                    chain_protocol_contracts
                        .get_mut(config.protocol_name)
                        .unwrap()
                        .insert(name.clone(), contract.address.clone());
                    found_contracts
                        .get_mut(config.protocol_name)
                        .unwrap()
                        .insert(contract_name);
                    matched = true;
                    break;
                }
            }

            if !matched {
                debug!(
                    contract = %name,
                    chain_id = %chain_id,
                    "Contract not assigned to any protocol"
                );
            }
        }

        for config in PROTOCOL_CONFIGS {
            let contracts = chain_protocol_contracts.get(config.protocol_name).unwrap();
            if !contracts.is_empty() {
                protocol_chains
                    .get_mut(config.protocol_name)
                    .unwrap()
                    .insert(chain_id, contracts.clone());
            }
        }
    }

    for config in PROTOCOL_CONFIGS {
        let found = found_contracts.get(config.protocol_name).unwrap();
        let missing: Vec<String> = config
            .contracts
            .iter()
            .filter(|contract| !found.contains(*contract))
            .map(|s| s.to_string())
            .collect();

        if !missing.is_empty() {
            return Err(ParseError::MissingContracts {
                protocol_name: config.protocol_name.to_string(),
                contracts: missing,
            });
        }
    }

    let mut result = Vec::new();
    for config in PROTOCOL_CONFIGS {
        let chains = protocol_chains.get(config.protocol_name).unwrap();
        if !chains.is_empty() {
            result.push(ProtocolDeployments {
                protocol_name: config.protocol_name.to_string(),
                chains: chains.clone(),
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uniswap() {
        let path = "source/uniswap/deployments";
        let res = parse(path);
        assert!(res.is_ok());

        let protocols = res.unwrap();
        assert!(!protocols.is_empty());

        for protocol in &protocols {
            assert!(
                protocol.protocol_name == "uniswap-v2"
                    || protocol.protocol_name == "uniswap-v3"
                    || protocol.protocol_name == "uniswap-v4"
                    || protocol.protocol_name == "universal-router"
                    || protocol.protocol_name == "permit2"
            );
            assert!(!protocol.chains.is_empty());

            for (chain_id, contracts) in &protocol.chains {
                assert!(chain_id > &0);
                assert!(!contracts.is_empty());
            }
        }
    }

    #[test]
    fn test_parse_uniswap_specific_chains() {
        let path = "source/uniswap/deployments";
        let res = parse(path);
        assert!(res.is_ok());

        let protocols = res.unwrap();

        for protocol in &protocols {
            if protocol.protocol_name == "uniswap-v2" {
                assert!(protocol.chains.contains_key(&1));
                let mainnet_contracts = protocol.chains.get(&1).unwrap();
                assert!(!mainnet_contracts.is_empty());
                assert!(mainnet_contracts.contains_key("UniswapV2Factory"));
            }

            if protocol.protocol_name == "uniswap-v3" {
                assert!(protocol.chains.contains_key(&1));
                let mainnet_contracts = protocol.chains.get(&1).unwrap();
                assert!(!mainnet_contracts.is_empty());
                assert!(mainnet_contracts.contains_key("UniswapV3Factory"));
            }
        }
    }
}
