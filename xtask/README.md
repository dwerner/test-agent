# xtask Usage Guide

This guide will help you understand how to use the various commands in the `xtask` project.

## Available Commands

1. `cargo xtask fmt-lint`
    This command formats and lints the code in the project using `cargo fmt` and `cargo clippy`.

    Usage:

    ```sh
    cargo xtask fmt-lint
    ```

2. `cargo xtask build-all`
    This command builds all binaries in the project (client and server).

    Usage:

    ```sh
    cargo xtask build-all
    ``` 
3. `cargo xtask run-daemon`
    This command runs just the daemon, which is useful for testing.

    Usage:

    ```sh
    cargo xtask run-daemon
    ```
4. `cargo xtask generate-self-signed-cert [hostname]`
    This command generates a self-signed certificate and key for the agent, given a hostname.

    Usage:

    ```sh
    cargo xtask generate-self-signed-cert [hostname]
    ```

    Replace `[hostname]` with the desired hostname for the certificate. 
5. `cargo xtask dist [version] [--regenerate-key-and-certificate]`
    This command creates a distribution tarball of the agent, with a given version number provided (manual). Optionally, it can also regenerate the key and certificate.

    Usage:

    ```sh
    cargo xtask dist [version] [--regenerate-key-and-certificate]
    ```

    Replace `[version]` with the desired version number for the tarball. If you want to regenerate the key and certificate, add the `--regenerate-key-and-certificate` flag. 
6. `cargo xtask clean-dist`
    This command cleans the dist directory.

    Usage:

    ```sh
    cargo xtask clean-dist
    ```
