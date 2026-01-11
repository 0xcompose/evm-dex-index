mod balancer;
mod types;
mod write;

const TARGET_FOLDER: &str = "deployments";

const BALANCER_REPO_PATH: &str = "source/balancer";

fn main() {
    let (v2_deployments, v3_deployments) =
        balancer::parse(BALANCER_REPO_PATH).expect("Failed to parse balancer deployments");

    write::write(TARGET_FOLDER, v2_deployments).expect("Failed to write v2 deployments");
    write::write(TARGET_FOLDER, v3_deployments).expect("Failed to write v3 deployments");
}
