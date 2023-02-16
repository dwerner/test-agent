# xcasper

This intended to be a tool to wrap test-agent in a casper-specific way. Any casper-node related functionality should be here, calling to the agent's daemon with the client whenever needed to manipulate the service and file systems. In this way, we can abstract away platform specific functionality and still have a specific implementation here.

Goals:

- network asset generation
- mainnet dump integration into a generated network
- compilation and distribution packaging for the node and launcher
- staging of upgrades on a given set of nodes
- interaction with ec2 to bring up/down instances with particular characteristics 

This tool was based on xtask, so it's called in a similar manner:

```
cargo xcasper <subcommand> <args>
```

TODO enumerate command examples