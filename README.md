# Oracle Core
The oracle core is the off-chain component that oracles who are part of an oracle pool must run. This oracle core provides a HTTP api interface for submitting datapoints and will automatically generate/post transactions. This thereby allows the oracle to participate in the oracle pool protocol without any extra work by the oracle operator.

The current design does not include bootstrapping of the oracle pool. This must be done separately.

Do note, that the oracle core requires the user to have access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools, a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).


## Roadmap
1. Define basic requirements/structure of the oracle core.
2. Implement functions for all core <-> full node interactions.
3. Build tiny CLI which allows for manual testing of oracle pool protocol using the interaction functions.
4. Build automated logic into the oracle core for interacting with the oracle pool on-chain/building txs. (Using placeholder data)
5. Build the HTTP API which operators can use to submit data & check the status of their oracle core.


## HTTP API Endpoints

### GET

#### blockHeight
Returns the current block height of the Ergo blockchain.

#### oracleInfo
Returns json with information about the local oracle:
- Oracle address

#### poolInfo
Returns json with information about the oracle pool:
- Contract addresses
- Posting Price
- Live Epoch Length
- Epoch Preparation Length
- Margin Of Error
- Oracle Pool NFT
- Oracle Pool Participant Token

#### nodeInfo
Returns json with information about the node that the oracle is using:
- Node url

#### poolStatus
Returns the current status of the oracle pool.
- Funded Percentage (Total Funds/Pool Price * 100)
- Current Pool State (Epoch Preparation Vs. Live Epoch)


#### oracleStatus
Returns the current status of one's own oracle.
- Waiting For Datapoint To Be Submitted To The Core For The Current Epoch (True/False)
- Latest Datapoint
- Epoch ID That the Datapoint Was Submit In
- Creation Height Of The Datapoint Tx


### POST

#### submitDatapoint
Example Json Body:
```json
{
    datapoint: 123456
}
```


Allows the owner of an oracle to commit a datapoint for the current running epoch. If the pool is in the epoch preparation stage, the datapoint will be rejected. The provided datapoint must be a valid integer.



## Oracle Pool Config
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



### Oracle Core <-> Node Interaction
Using the new [EIP-1](https://github.com/ergoplatform/eips/blob/master/eip-0001.md), the oracle core will register scans to find all relevant boxes.

The initial implementation includes scans for:

1. A box at the `Datapoint` contract address which also contains the oracle's address in R4 and the oracle pool participant token.
2. Boxes at the `Datapoint` contract address and the oracle pool participant token.
3. Any boxes at the `Pool Deposit` contract address.
4. A box at the "Epoch Preparation" contract address which holds the oracle pool NFT.
5. A box at the "Oracle Pool Epoch" contract address which holds the oracle pool NFT.

The oracle core saves each of the `scanId`s locally into `scanIDs.json` after registering them with the full node. At any time the oracle core wishes to check the current state of the protocol, it simply reads the `scanId`s and acquires all of the relevant unspent boxes from the node.


### Transaction Building
The oracle core creates the following action transactions within the basic oracle pool protocol:

1. Commit Datapoint
2. Collect Datapoints
3. Collect Funds
4. Start Next Epoch
5. Create New Epoch