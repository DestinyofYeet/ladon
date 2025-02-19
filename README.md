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

PS: Sorry for the Arc<Mutex<T>> hell

# Todos
- [ ] Fronted (Yew?)
- [ ] Capture Derivation build output and store it in db
