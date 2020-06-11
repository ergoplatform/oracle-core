# Oracle Core
The off-chain component that oracles who are part of an oracle pool must run. This oracle core provides an HTTP interface for submitting datapoints to and will automatically generate and post transactions, thereby participating in the oracle pool protocol without any extra work by the oracle.

Do note, that the oracle core requires the user to have access to a full node in order to perform UTXO-set scanning.




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


