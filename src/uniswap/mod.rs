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
struct UniswapDeployment {
    #[serde(rename = "chainId")]
    chain_id: String,
    latest: HashMap<String, ContractDeployment>,
}

#[derive(Debug, Deserialize)]
struct ContractDeployment {
    address: String,
}

type ProtocolName = &'static str;

type ProtocolsDeployments = HashMap<ProtocolName, ChainDeployments>;

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

fn parse_chain_id(chain_id_str: &str) -> Result<u64, std::io::Error> {
    chain_id_str.parse().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid chain_id: {}", chain_id_str),
        )
    })
}

fn validate_protocol_configs_for_duplicate_definitions() -> Result<(), ParseError> {
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

fn build_response(
    protocol_chains: HashMap<&str, ChainDeployments>,
) -> Result<Vec<ProtocolDeployments>, ParseError> {
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

fn try_to_find_missing_contracts(protocol_chains: &ProtocolsDeployments) -> Result<(), ParseError> {
    for config in PROTOCOL_CONFIGS {
        let chains: &HashMap<u64, ChainContracts> = protocol_chains
            .get(config.protocol_name)
            .expect("Protocol not found");

        let mut found_contracts: HashSet<&str> = HashSet::new();

        for (_chain_id, contracts) in chains {
            for contract_name in contracts.keys() {
                found_contracts.insert(contract_name.as_str());
            }
        }

        let missing: Vec<String> = config
            .contracts
            .iter()
            .filter(|&&contract| !found_contracts.contains(contract))
            .map(|s| s.to_string())
            .collect();

        if !missing.is_empty() {
            return Err(ParseError::MissingContracts {
                protocol_name: config.protocol_name.to_string(),
                contracts: missing,
            });
        }
    }

    Ok(())
}

fn read_deployments(path_to_deployments: &str) -> Result<Vec<UniswapDeployment>, std::io::Error> {
    let entries = std::fs::read_dir(path_to_deployments)?;
    let mut deployments = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let deployment: UniswapDeployment =
            serde_json::from_reader(BufReader::new(File::open(&path)?))?;
        deployments.push(deployment);
    }
    Ok(deployments)
}

fn init_protocol_chains() -> ProtocolsDeployments {
    let mut protocol_chains: ProtocolsDeployments = HashMap::new();

    for config in PROTOCOL_CONFIGS {
        protocol_chains.insert(config.protocol_name, HashMap::new());
    }

    protocol_chains
}

pub fn parse(path_to_deployments: &str) -> Result<Vec<ProtocolDeployments>, ParseError> {
    validate_protocol_configs_for_duplicate_definitions()?;

    let mut protocol_chains: ProtocolsDeployments = init_protocol_chains();

    let deployments = read_deployments(path_to_deployments)?;

    for chain_deployments in deployments {
        let chain_id: u64 = parse_chain_id(&chain_deployments.chain_id)?;

        let mut chain_protocol_contracts: HashMap<ProtocolName, ChainContracts> = HashMap::new();

        for config in PROTOCOL_CONFIGS {
            chain_protocol_contracts.insert(config.protocol_name, ChainContracts::new());
        }

        for (name, contract) in chain_deployments.latest {
            let mut matched = false;

            for config in PROTOCOL_CONFIGS {
                if !config.contracts.iter().any(|&c| c == name.as_str()) {
                    continue;
                }

                chain_protocol_contracts
                    .get_mut(config.protocol_name)
                    .expect("Not found protocol")
                    .insert(name.clone(), contract.address.clone());

                matched = true;
                break;
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
                    .insert(chain_id, contracts.to_owned());
            }
        }
    }

    try_to_find_missing_contracts(&protocol_chains)?;

    let result = build_response(protocol_chains)?;

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
