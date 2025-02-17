use evaluator::Evaluator;


mod evaluator;

#[tokio::main]
async fn main() {
    let mut eval = Evaluator::new(
        "/home/ole/nixos",
        // "hydraJobs"
        r#"nixosConfigurations."main".config.system.build.toplevel"#
    );

    eval.start().await;
}
