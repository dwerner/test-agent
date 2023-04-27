# Test Agent

Opinionated monitoring and debug tool to enable debugging locally and remotely.

1. *Compile binaries for the network build:
    - `cargo xcasper compile -p casper-node --release` one by one OR
    - `cargo xcasper compile-all-projects -c compile.yaml` use the compile.yaml to build based on some predefined critera (existing checkouts, etc). See `cargo xcasper compile-all-projects --help` for more options.
2. Generate network config
    - `cargo xcasper gen-net-config <name-of-network> default` use the default values to generate a network config. See `cargo xcasper gen-net-config --help` for more options.
3. Copy assets to network dir
    - `cargo xcasper copy-artifacts-to-network-dir <name-of-network>` use the config.yaml to copy the assets to the network dir. See `cargo xcasper copy-artifacts-to-network-dir --help` for more options.
4. Build the agent
    - `cargo xtask build-all --release` build the daemon and client.
    - `cargo xtask dist <agent-version>`

TODO: complete the deployment steps

## Self Update
todo

# Dependencies
todo

# Provisioning

# User Stories
As a developer of the node software (client, launcher, node, etc), I want to be able to: 
- create a network.
- include mainnet data in that network.
- include my development workstation as a node in a network.

# subgoals
- allow provisioning of assets, separate from initialization of network: we should allow node provisioning to happen and make the activation point be configurable
- allow the shutdown of a network at a switch block, allowing the network to come back up all-together.
