use evaluator::Evaluator;


mod evaluator;

#[tokio::main]
async fn main() {
    let mut eval = Evaluator::new(
        "git+https://git.ole.blue/ole/nix-config",
        // "hydraJobs"
        r#"nixosConfigurations."nix-server".config.system.build.toplevel"#
    );

    eval.start().await;
}
