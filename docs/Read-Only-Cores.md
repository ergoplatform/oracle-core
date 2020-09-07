Read Only Oracle Cores
===============

There are several scenarios where it may be useful to have access to the current state of a given oracle pool via running one's own oracle core. Whether this is for greater redundancy for the frontend, reading the state directly for local use within your own off-chain application, or anything else in between, having the ability to use the oracle core as an on-chain data parsing utility is valuable.


As such the oracle core has a simple flag to enable this feature `--readonly`. To enable "read only" mode, use the flag on launch of the oracle core as such:
```sh
./oracle-core --readonly
```