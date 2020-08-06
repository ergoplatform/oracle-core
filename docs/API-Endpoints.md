## HTTP API Endpoints
The oracle core runs two small local HTTP API servers.

The GET API server allows oracle connectors/external clients to read the current state of the oracle/oracle pool. The port of this API server can be safely made publicly accessible as it does not provide any endpoints (such as datapoint posting) that can cause trouble/effect the state of the protocol. The GET API server uses the `api-port` defined in `oracle-config.yaml`.

The POST API server allows oracle connectors to submit datapoints. It uses the very next port after the one that the GET API server is using (ie. GET = 9090, PUT = 9091). The port of the POST API server should never be opened/made publicly accessible because it is a major security threat.

### GET API

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


### POST API

#### submitDatapoint
Example Json Body:
```json
{
    datapoint: 123456
}
```


Allows the owner of an oracle to commit a datapoint for the current running epoch. If the pool is in the epoch preparation stage, the datapoint will be rejected. The provided datapoint must be a valid integer.


