# Oracle Core
The oracle core is the off-chain component that oracles who are part of an oracle pool must run. This oracle core provides an HTTP interface for submitting datapoints to and will automatically generate and post transactions, thereby participating in the oracle pool protocol without any extra work by the oracle.

The current design does not include bootstrapping of the oracle pool. This must be done separately.

Do note, that the oracle core requires the user to have access to a full node in order to perform UTXO-set scanning. Also, each oracle core is designed to work with a single oracle pool. If an operator runs several oracles in several oracle pools, a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).




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
Each operator must set up their `oracle-config.yaml` with information about their oracle and the oracle pool they are taking part in.

- Oracle address (address of the oracle which must be in R4 of the datapoint box and owned in the full node wallet)
- The type of the oracle pool datapoint. Current options: `[Int, String]`
- Oracle Pool NFT/Singleton Token ID (Token which always stays in the oracle pool box)
- Oracle Pool Participant Token ID (Token which is held in the oracle's datapoint box)
- *Epoch Preparation* Stage Contract Address
- *Oracle Pool Epoch* Stage Contract Address
- *Datapoint* Stage Contract Address
- *Pool Deposit* Stage Contract Address
- Stake Slashing (Boolean for now, set to false as no support in initial version)
- Governance Voting (Boolean for now, set to false as no support in initial version)



### Oracle Core <-> Node Interaction
Using the new [EIP-1 Application-Friendly Wallet API](https://github.com/ergoplatform/eips/blob/master/eip-0001.md), the oracle core will register scans to find all of the relevant boxes.

For the initial implementation these scans include:

1. A box in the `Datapoint` contract address which also contains the oracle's address in R4.
2. Any boxes at the `Pool Deposit` contract address.
3. A box in the "Epoch Preparation" contract address which holds the oracle pool NFT.
4. A box in the "Oracle Pool Epoch" contract address which holds the oracle pool NFT.

The oracle core will save each of the `scanId`s locally after registering them with the full node. At any time the oracle core wishes to check the current state of the protocol, it simply uses the `scanId`s to acquire all of the relevant boxes.


### Transaction Building
