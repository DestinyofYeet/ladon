{ rustPlatform, lib, pkgs, ... }:

rustPlatform.buildRustPackage {
  pname = "pkg";
  version = "1.0";

  src = ./.;

  nativeBuildInputs = with pkgs; [ cargo-leptos lld binaryen ];

  cargoHash = "";
  useFetchCargoVendor = true;

  buildPhase = ''
    cargo leptos build --release
  '';

  meta = with lib; {
    description = "A program";
    license = licenses.gpl2;
    platforms = platforms.all;
  };
}
