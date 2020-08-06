# Oracle Config
Each operator must set up their `oracle-config.yaml` with information about their address, the node they are using, and the oracle pool they are taking part in.

- IP Address of the node (default is local)
- Port that the node is on (default is 9053)
- Node API key
- Oracle address (address of the oracle which must be in R4 of the datapoint box and owned in the full node wallet)
- Oracle Pool NFT/Singleton Token ID (Token which always stays in the oracle pool box)
- Oracle Pool Participant Token ID (Token which is held in the oracle's datapoint box)
- *Epoch Preparation* Stage Contract Address
- *Oracle Pool Epoch* Stage Contract Address
- *Datapoint* Stage Contract Address
- *Pool Deposit* Stage Contract Address
- Live Epoch Length
- Epoch Preparation Length
- Stake Slashing (Boolean for now, set to false as no support in initial version)
- Governance Voting (Boolean for now, set to false as no support in initial version)
- Etc.


