# Oracle Core
The oracle core is the off-chain component that oracles who are part of an oracle pool must run. This oracle core provides a HTTP interface for submitting datapoints and will automatically generate/post transactions thereby participating in the oracle pool protocol without any extra work by the oracle operator.

The current design does not include bootstrapping of the oracle pool. This must be done separately.

Do note, that the oracle core requires the user to have access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools, a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).


## Roadmap
1. Define basic requirements/structure of the oracle core.
2. Implement functions for all core <-> full node interactions.
3. Build tiny CLI which allows for manual testing of oracle pool protocol using the interaction functions.
4. Build automated logic into the oracle core for interacting with the oracle pool on-chain/building txs. (Using placeholder data)
5. Build the HTTP API which operators can use to submit data & check the status of their oracle core.


## Initial Design Notes


### Potential HTTP API Endpoints

#### GET

##### poolStatus
Returns the current status of the oracle pool.
- Funded Percentage (Total Funds/Pool Price * 100)
- Current Pool State (Preparing For Epoch Vs. Active Epoch)
- Latest Epoch Box ID


##### oracleStatus
Returns the current status of one's own oracle.
- Has Commit Datapoint In Latest Epoch (True/False)
- Latest Datapoint (a tuple of `(Epoch Box ID, Block Height Submitted, Datapoint Value)`)
- (Collateral info when that is implemented)


#### POST

##### submitDatapoint
Allows the owner of an oracle to commit a datapoint for the current running epoch. If the pool is in the epoch preparation stage, the datapoint will be rejected. The provided datapoint must be parsable into the type expected which is set in the oracle pool config.



### Oracle Pool Config
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



### Oracle Core <-> Node Interaction
Using the new [EIP-1](https://github.com/ergoplatform/eips/blob/master/eip-0001.md), the oracle core will register scans to find all relevant boxes.

For the initial implementation these scans include:

1. A box in the `Datapoint` contract address which also contains the oracle's address in R4.
2. Any boxes at the `Pool Deposit` contract address.
3. A box in the "Epoch Preparation" contract address which holds the oracle pool NFT.
4. A box in the "Oracle Pool Epoch" contract address which holds the oracle pool NFT.

The oracle core saves each of the `scanId`s locally into `scanIDs.json` after registering them with the full node. At any time the oracle core wishes to check the current state of the protocol, it simply reads the `scanId`s from the file and acquire all of the relevant unspent boxes from the node.


### Transaction Building

The oracle core will have to build transactions for the following actions which are possible in the basic oracle pool protocol.

1. Commit Datapoint
2. Collect Datapoints
3. Fund Oracle Pool
4. Collect Funds
5. Start Next Epoch
6. Create New Epoch

Reference the informal specification for details on building said transactions.
