# xcasper

xcasper is a command-line tool for building and managing Casper projects. It provides functionalities to compile Rust projects, generate network configuration assets, copy artifacts to the network directory, and stage upgrades.

This intended to be a tool to wrap test-agent in a casper-specific way. Any casper-node related functionality should be here, calling to the agent's daemon with the client whenever needed to manipulate the service and file systems. In this way, we can abstract away platform specific functionality and still have a specific implementation here.

Goals:

- network asset generation
- mainnet dump integration into a generated network
- compilation and distribution packaging for the node and launcher
- staging of upgrades on a given set of nodes
- interaction with ec2 to bring up/down instances with particular characteristics 

This tool was based on xtask, so it's called in a similar manner:

Usage

The xcasper tool provides two subcommands to compile Rust projects: Compile and CompileAllProjects.
Compile a Single Rust Project

The Compile subcommand allows you to compile a single Rust project. It takes three arguments:

- project: The short name of the project you want to compile (`casper-node`, `casper-client`, `casper-db-utils`, `global-state-update-gen`, c)
- existing_checkout: (Optional) The path to an existing checkout of the project. If not provided, the tool will check out the project for you.
- debug: (Optional) A flag to compile the project in debug mode. If not provided, the project will be compiled in release mode.


To generate network configuration assets:

```bash
cargo xcasper gen-network-config
```
To copy artifacts to the network directory:

```bash
cargo xcasper copy-artifacts-to-network-dir
```
