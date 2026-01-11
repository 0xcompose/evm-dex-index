mod balancer_v2;
mod parser;

const BALANCER_REPO_PATH: &str = "source/balancer";

fn main() {
    let deployments =
        balancer_v2::parse(BALANCER_REPO_PATH).expect("Failed to parse balancer v2 deployments");
}
