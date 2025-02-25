# Hydra-rs

This project is supposed to be an alternative to the official [Hydra Project](https://github.com/NixOS/hydra).

This is still a on-going thing and the current way of making it build something is not as clean as the official one, since hydra-rs calls 'nix build' directly, so it has to be pointed
to a evaluable attribute and not an attrset like hydraJobs.

# Current developement state

You can run it with

```bash
mkdir tmp
cargo run -- -d ./tmp
```

This will create a sqlite db in ./tmp.

Although the flake path and attributes are still hardcoded in main.rs :)

PS: Sorry for the Arc<Mutex<\T>> hell

# Todos
- [ ] Capture Derivation build output and store it in db
- [ ] Currently a single action can result in multiple derivations. Each derivaition calls back after a build on its own, setting the value of action twice
- [ ] Fronted (Yew?)

## Idea

1. Call `nix eval flakeUri --json` in order to get json output. For example:
```bash
nix eval nix eval /home/ole/nixos#hydraJobs --json
```

2. Call `nix derivation show /nix/store/...` on whatever store path(s) were printed out by `nix eval`. Parse the resulting json and get the .drv path from the json key.

3. Call `nix-store --realise /nix/store/....drv` which actually builds the derivation
