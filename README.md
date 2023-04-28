# Test Agent Tools

## Vision

### Caveat Emptor
The goal of this project is to provide a set of tools that can be used to build, manage, and interact with the Casper Network projects _in a testing context_. The tools are designed to be used by developers, testers, and other stakeholders to perform various tasks in the development lifecycle of the node, running system tools, and interacting with the ad-hoc network. These tools are intended to be used in a test environment where the operator has full control over the network. This is not intended for production use, and is not supported as a general-purpose set of tools for interacting with the Casper Network.

### Goals

The goal of this project is to enable develpers and testers to:

1. Build a network of nodes (optionally porting data from an existing network and patching the validator set)
1. Run deploys and contracts on the network produced
1. Stage and perform upgrades on the network
1. Run integration and load testing on the network

In order to accomplish these goals, several things are needed:

1. Assets for the network
    - config.toml
    - chainspec.toml
    - accounts.toml (which is patched into the chainspec.toml)
    - public and private keys for all nodes
    - binaries: casper-node, casper-node-launcher, related tools
1. A way to manage the network
    - provision remotes, deploy agent and assets to target remotes (tbd)
    - start/stop nodes (client start-service/stop-service)
    - stage upgrades (client put-file)
    - run deploys (casper-client)
    - run tests (casper-test)

Assets are generated using the `xcasper` tool, which includes building the binaries and configuration needed. Once generated, a package needs to be deployed to a remote which containing the daemon and network node assets.

## Components

This is a collection of tools designed to work together in order to build, manage, and interact with the Casper Network projects. The main components of this collection are:

1. client [bin/client/README.md](bin/client/README.md)
1. daemon [bin/daemon/README.md](bin/daemon/README.md)
1. xcasper [xcasper/README.md](xcasper/README.md)
1. xtask [xtask/README.md](xtask/README.md)
1. agent-lib [crates/agent-lib/README.md](crates/agent-lib/README.md)

More detailed information about each component can be found below, or in the README.md files for each component.

### Agent Lib

The agent lib is a library that provides a common interface for interacting with the Casper Network. It is used by the client and daemon to communicate with each other. See the [agent lib README.md](crates/agent-lib/README.md) for more information.

### client

The `client` is an RPC client that communicates directly with instances of the daemon, providing an interface to interact with the Casper Network nodes. See the [client README.md](bin/client/README.md) for more information.

### daemon

The `daemon` is a sidecar process designed to run on the same machine as a Casper Network node. It works in tandem with the `client` to perform various tasks such as starting/stopping nodes, running system tools, etc. See the [daemon README.md](bin/daemon/README.md) for more information.

### xcasper

`xcasper` is the main tool that handles building all related projects, generating network assets, and managing versioning for staging upgrades. It optionally uses a `compile.yaml` file to define existing checkouts and other options. The tool provides subcommands to compile individual projects or all projects at once.
 
See the [xcasper README.md](xcasper/README.md) for more information.

### xtask

The `xtask` project is responsible for some packaging tasks, particularly building a distribution package of the `client` and `daemon`. See the [xtask README.md](xtask/README.md) for more information.

1. *Compile binaries for the network build:
    - `cargo xcasper compile -p <project> --release` one by one OR
    - `cargo xcasper compile-all-projects -c compile.yaml` use the compile.yaml to build based on some predefined critera (existing checkouts, etc). See `cargo xcasper compile-all-projects --help` for more options.
2. Generate network config
    - `cargo xcasper gen-net-config <name-of-network> default` use the default values to generate a network config. See `cargo xcasper gen-net-config --help` for more options.
3. Copy assets to network dir
    - `cargo xcasper copy-artifacts-to-network-dir <name-of-network>` use the config.yaml to copy the assets to the network dir. See `cargo xcasper copy-artifacts-to-network-dir --help` for more options.
4. Build the agent
    - `cargo xtask build-all --release` build the daemon and client.
    - `cargo xtask dist <agent-version>`
