use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs::File,
    io::BufReader,
};

use serde::Deserialize;
use thiserror::Error;

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
    InvalidChainId { chain_id: String, file_name: String },

    IoError(#[from] std::io::Error),

    SerdeError(#[from] serde_json::Error),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn parse(path_to_deployments: &str) -> Result<ProtocolDeployments, ParseError> {
    let mut chains: ChainDeployments = HashMap::new();

    let entries = std::fs::read_dir(path_to_deployments)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let deployment: ChainDeployment = serde_json::from_reader(reader)?;

        let chain_id: u64 =
            deployment
                .chain_id
                .parse()
                .map_err(|_| ParseError::InvalidChainId {
                    chain_id: deployment.chain_id.clone(),
                    file_name: file_name.clone(),
                })?;

        let mut contracts: ChainContracts = ChainContracts::new();

        for (name, contract) in deployment.latest {
            contracts.insert(name, contract.address);
        }

        chains.insert(chain_id, contracts);
    }

    Ok(ProtocolDeployments {
        protocol_name: "uniswap".to_string(),
        chains,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uniswap() {
        let path = "source/uniswap/deployments";
        let res = parse(path);
        assert!(res.is_ok());

        let deployments = res.unwrap();
        assert_eq!(deployments.protocol_name, "uniswap");
        assert!(!deployments.chains.is_empty());

        for (chain_id, contracts) in deployments.chains {
            assert!(chain_id > 0);
            assert!(!contracts.is_empty());
        }
    }

    #[test]
    fn test_parse_uniswap_specific_chains() {
        let path = "source/uniswap/deployments";
        let res = parse(path);
        assert!(res.is_ok());

        let deployments = res.unwrap();

        assert!(deployments.chains.contains_key(&1));
        assert!(deployments.chains.contains_key(&8453));

        let mainnet_contracts = deployments.chains.get(&1).unwrap();
        assert!(!mainnet_contracts.is_empty());
    }
}
