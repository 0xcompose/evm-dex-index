use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProtocolDeployments {
    pub protocol_name: String,
    pub chains: ChainDeployments,
}

pub type ChainId = u64;

pub type ChainDeployments = HashMap<ChainId, ChainContracts>;

pub type ChainContracts = BTreeMap<ContractName, ContractAddress>;

pub type ContractName = String;

pub type ContractAddress = String;
