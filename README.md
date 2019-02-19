# zincati

Cincinnati to rpm-ostree update agent.

This program contains an update-management agent which bridges between a cincinnati server and rpm-ostree daemon, implementing conditional strategies for finalization.

It is made of three actors passing action-requests to each other:
 * An update agent, with support for several user-strategies.
 * An HTTP client to Cincinnati, periodic scraper.
 * A DBus client for rpm-ostree daemon.

## Example

```
RUST_LOG=zincati=trace cargo run
```
