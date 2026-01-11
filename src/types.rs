use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProtocolDeployments {
    pub protocol_name: String,
    pub chains: ChainDeployments,
}

pub type ChainId = u64;

pub type ChainDeployments = HashMap<ChainId, ChainContracts>;

pub type ChainContracts = HashMap<ContractName, ContractAddress>;

pub type ContractName = String;

pub type ContractAddress = String;

#[derive(Debug, Serialize)]
pub struct ProtocolDeployment {
    pub name: String,
    pub chain_id: ChainId,
    pub contracts: HashMap<String, String>,
}

impl ProtocolDeployment {
    pub fn new(name: &str, chain_id: u64, contracts: HashMap<String, String>) -> Self {
        Self {
            name: name.to_string(),
            chain_id,
            contracts,
        }
    }
}
