{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, ... }@inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        name = "ladon";
        version = "1.0";

        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = inputs.crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src version;
          pname = name;
          buildInputs = with pkgs; [ cargo-leptos binaryen lld ];
        };

        artifacts = craneLib.buildDepsOnly commonArgs;

        buildArgs = commonArgs // {
          cargoArtifacts = artifacts;
          buildPhaseCargoCommand = "cargo leptos build --release -vvv";

          installPhaseCommand = ''
            mkdir -p $out/bin
            cp target/server/release/${name} $out/bin/
            cp -r target/site $out/bin/
            wrapProgram $out/bin/${name} \
              --set LEPTOS_SITE_ROOT $out/bin/site
          '';
        };

        package = craneLib.buildPackage buildArgs;

      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            # openssl
            # pkg-config
            rust-analyzer
            cargo-leptos
            lld
            dart-sass
            sqlite.dev
            sqlx-cli
            rustfmt
            stylance-cli
            clippy
            vscode-langservers-extracted
            binaryen
          ];

          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          DATABASE_URL = "sqlite:tmp/db.sqlite";
          # shellHook = ''
          #   if [ -f ".direnv/stylance_pid" ]; then
          #     pid=$(cat ".direnv/stylance_pid")
          #     if ps -p $pid > /dev/null
          #     then
          #       echo "Stylance is running: $pid"
          #     else
          #       _=$(stylance -w ./ & echo $! > ".direnv/stylance_pid")&
          #   else
          #     _=$(stylance -w ./ & echo $! > ".direnv/stylance_pid")&
          #   fi
          # '';
        };

        packages.default = package;
      });
}
