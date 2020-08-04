# Oracle Core

![](images/oracle-core.png)

The oracle core is the off-chain component that oracles who are part of an oracle pool run. This oracle core provides a HTTP API interface for submitting datapoints and will automatically generate/post transactions. This thereby allows the oracle to participate in the oracle pool protocol without any extra work by the oracle operator.

The oracle core requires that the user has access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools then a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).

A `connector` must also be used with the oracle core in order to acquire data to submit to the pool. Each connector sources data from the expected sources, potentially applies functions to said data, and then submits the data to the oracle core via HTTP API during the `Live Epoch` stage in the oracle pool protocol.


# Running An Oracle

1. Ensure your Ergo node is running with an unlocked wallet.
2. Edit the `oracle-config.yaml` to contain your oracle address (from the wallet), node config details, and all of the required parameters for your given oracle pool.
3. Launch the oracle core by running the executable `./oracle-core`.
4. Launch an oracle connector which fetches data and submit it to the oracle core automatically.
5. Use one of the API endpoints to check the status of your oracle.


# Bootstrapping An Oracle Pool

1. Generate a new NFT for your new Oracle Pool.
2. Generate a new "Oracle Pool Participant Token" of total count equal to the number of oracles who will be a part of the pool.
3. Decide on and set the key parameters of the pool which will be hard-coded into the contracts. (ie. Epoch Prep Length, Live Epoch Length, Payout Amount, etc.)
4. Compile the smart contracts using the parameters/token ids to acquire the smart contract/stage addresses.
5. Fill out a `oracle-config.yaml` with all of the parameters of the oracle pool which were set, as well as with the newly generated smart contract addresses.
6. Send the `oracle-config.yaml` to all of the oracles part of the pool.
7. Wait until all of the oracles launch the oracle core so that their nodes can register the proper UTXO-set scans.
8. Bootstrap the oracle pool box at the "Epoch Preparation" stage/contract, and holding the "Oracle Pool NFT".
9. Acquire the addresses of all of the oracles.
10. Bootstrap a "Datapoint" box for every single one of the oracles with their corresponding addresses held in R4, and with a single "Oracle Pool Participant Token".


## HTTP API Endpoints
The oracle core runs a small local HTTP API server which uses the `api-port` parameter in `oracle-config.yaml`. This API allows oracle connectors to read the current state of the oracle/oracle pool and submit datapoints.

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



### Oracle Pool Bootstrap Flow

1. Generate Pool NFT
2. Generate Participant Tokens
3. Get list of oracle addresses.
4. Set parameters in contracts.
5. Compile Contracts.
6. Register scans in node via oracle-cores for each oracle using compiled contract addresses.
7. Bootstrap "Epoch Prep" pool box.
8. Bootstrap oracle "Datapoint" boxes.