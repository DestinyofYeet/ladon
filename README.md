# Hydra-rs

This project is supposed to be an alternative to the official [Hydra Project](https://github.com/NixOS/hydra).

Develpement is still on-going, but it's supposed to just be a drop-in replacement for hydra.

# Current developement state

You can run it with

```bash
mkdir tmp
sqlx database create
sqlx migrate run
cargo leptos watch -- -d ./tmp
```

This will create a sqlite db in ./tmp.

PS: Sorry for the Arc<Mutex<\T>> hell

# Todos
See: https://git.ole.blue/ole/hydra-rs/projects

## Idea

1. Call `nix eval flakeUri --json` in order to get json output. For example:
```bash
nix eval nix eval /home/ole/nixos#hydraJobs --json
```

2. Call `nix derivation show /nix/store/...` on whatever store path(s) were printed out by `nix eval`. Parse the resulting json and get the .drv path from the json key.

3. Call `nix-store --realise /nix/store/....drv` which actually builds the derivation
