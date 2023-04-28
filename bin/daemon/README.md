# Test Agent RPC Daemon

## Summary

The daemon is a command-line tool for managing a remote node. It listens for incoming RPC calls from clients and performs various operations such as starting and stopping the service, fetching and putting files, and handling chunked file requests.


The following subcommand is used to start the daemon:

- `serve`: Start the daemon.

## Commands

### `serve`

Start the Agent RPC Server with the given address, certificate, and key:

```sh
daemon serve --addr <address> --cert <cert> --key <key>
```

- `--addr`: The address and port to bind the server to (default: "0.0.0.0:8081").
- `--cert`: The path to the certificate file (default: "assets/agent-crt.pem").
- `--key`: The path to the key file (default: "assets/agent-key.pem").

Usage

    Start the Agent RPC Server by running the following command:

```sh
daemon serve --addr 0.0.0.0:8081 --cert assets/agent-crt.pem --key assets/agent-key.pem
```

The server will listen for incoming connections on the specified address and port, and execute the requested operations.
