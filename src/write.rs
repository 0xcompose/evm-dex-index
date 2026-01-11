use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufWriter,
    path::Path,
};

use crate::types::ProtocolDeployment;

pub fn write(
    folder: &str,
    protocol_deployments: HashMap<u64, ProtocolDeployment>,
) -> Result<(), std::io::Error> {
    if !Path::new(folder).exists() {
        fs::create_dir_all(folder)?;
    }

    let protocol_path = format!(
        "{}/{}",
        folder,
        protocol_deployments.values().next().unwrap().name
    );

    if !Path::new(&protocol_path).exists() {
        fs::create_dir_all(&protocol_path)?;
    }

    for (chain_id, protocol_deployment) in protocol_deployments {
        let path = format!("{}/{}.json", protocol_path, chain_id);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &protocol_deployment)?;
    }

    Ok(())
}
