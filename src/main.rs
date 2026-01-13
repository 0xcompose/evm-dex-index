mod balancer;
mod types;
mod uniswap;
mod write;

const TARGET_FOLDER: &str = "deployments";

const BALANCER_REPO_PATH: &str = "source/balancer";
const UNISWAP_DEPLOYMENTS_PATH: &str = "source/uniswap/deployments";

fn main() {
    tracing_subscriber::fmt::init();

    let (v2_deployments, v3_deployments) =
        balancer::parse(BALANCER_REPO_PATH).expect("Failed to parse balancer deployments");

    write::write(TARGET_FOLDER, v2_deployments).expect("Failed to write v2 deployments");
    write::write(TARGET_FOLDER, v3_deployments).expect("Failed to write v3 deployments");

    let uniswap_deployments =
        uniswap::parse(UNISWAP_DEPLOYMENTS_PATH).expect("Failed to parse uniswap deployments");

    for deployment in uniswap_deployments {
        write::write(TARGET_FOLDER, deployment).expect("Failed to write uniswap deployment");
    }
}
