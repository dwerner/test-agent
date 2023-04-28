# Test Agent RPC Client

## Summary

The `client` is a command-line tool for interacting with the Agent Service. It allows you to perform various operations such as starting and stopping the service, fetching and putting files, and sending chunked file requests. A `network.yaml` file can be used to specify a list of peers to connect to.

The following subcommands are available:

- `start-service`: Ask the daemon to start a service on the remote.
- `stop-service`: Ask the daemon to stop a service on the remote.
- `fetch-file`: Ask the daemon to fetch a file from the remote.
- `put-file`: Put a file (monolithically) on the remote (zstd compressed on the fly).
- `put-file-chunked`: Put a file onto the remote in chunks (zstd compressed on the fly).

## Commands

### Start Service

```sh
client --daemon_peers <peers> --cert <cert> --key <key> start-service --id <service_id> --command <command> [--args <args>]


### Stop Service

```sh
client --daemon_peers <peers> --cert <cert> --key <key> stop-service --id <service_id>
```

### Fetch File

```sh
client --daemon_peers <peers> --cert <cert> --key <key> fetch-file --filename <filename>
```

### Put File

``` sh
client --daemon_peers <peers> --cert <cert> --key <key> put-file --source_file <source_file> --target_path <target_path>
```

### Put File Chunked
```sh
client --daemon_peers <peers> --cert <cert> --key <key> put-file-chunked --source_file <source_file> --target_path <target_path>
```
