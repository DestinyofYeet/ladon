{
  description = "A very basic flake";

  inputs = { nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable"; };

  outputs = { self, nixpkgs }@inputs:
    let pkgs = import nixpkgs { system = "x86_64-linux"; };
    in {
      devShells.x86_64-linux.default = pkgs.mkShell {
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

      packages.x86_64-linux.default = pkgs.callPackage ./pkg.nix { };
    };
}
