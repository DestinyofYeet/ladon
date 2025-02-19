# Hydra-rs

This project is supposed to be an alternative to the official [Hydra Project](https://github.com/NixOS/hydra).

This is still a on-going thing and the current way of making it build something is not as clean as the official one, since hydra-rs calls 'nix build' directly, so it has to be pointed
to a evaluable attribute and not an attrset like hydraJobs.
