use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProtocolDeployment {
    name: String,
    chain_id: u64,
    contracts: HashMap<String, String>,
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
