## HTTP API Endpoints
The oracle core runs two small local HTTP API servers.

The GET API server allows oracle connectors/external clients to read the current state of the oracle/oracle pool. The port of this API server can be safely made publicly accessible as it does not provide any endpoints (such as datapoint posting) that can cause trouble/effect the state of the protocol. The GET API server uses the `api-port` defined in `oracle-config.yaml`.

The POST API server allows oracle connectors to submit datapoints. It uses the very next port after the one that the GET API server is using (ie. GET = 9090, PUT = 9091). The port of the POST API server should never be opened/made publicly accessible because it is a major security threat.

### GET API

#### /blockHeight
Returns the current block height of the Ergo blockchain.

Example Response:
```json
289391
```

#### /oracleInfo
Returns json with information about the local oracle:
- Oracle address

Example Response:
```json
{
    "oracle_address": "9fj9NJpzo13HfNyCdzyfNP8zAfjiTY3pys1JP5wCzez8MiP8QbF"
}
```

#### /poolInfo
Returns json with information about the oracle pool:
- Contract addresses
- Oracle Payout Price
- Live Epoch Length
- Epoch Preparation Length
- Margin Of Error
- Oracle Pool NFT ID
- Oracle Pool Participant Token ID

Example Response:
```json
{
    "live_epoch_address": "F7f7vfC28mjet2UuH3cKyL7CHbtnc1AgxnEBQ6UhMvRtxhX7BrG7MLhMj7JEcmMyQqRzHg7hfoLNSzoDWg4PWfqSxoZXkTBPUWharJCtoRjaoHGYgjF9BJCjDNR13EwMVoXBhY2gmgfWyCjKjncFpjzbSBQYRAsj7W5vg3A2NtXudGMn2YjfHSqjFk1xzV4sfYGtfM9fLfd3ZEBMFfQpPRapG4DXaGL8emVrRsqfjGDqVRkxw1kJyffbFTsDStRgrKeGbA1gZKsKAYJiWLYVbmndRxhUuM7fQhtX8qzRMpDfqti43eotgxVXU5pr9Q7a4Pv2VbvS8gBDceRPZeLdsxBiDoWVbGEkF8vB7QrDNr9YxXEob4KircTpECARmcGgeLCHwr2i7AMGbs2tFFLX7PoHyYRv3ertFGS1CEth6wnjmo3SEjK8HXU",
    "epoch_prep_address": "Gxd4hMRT6J1SA6D3tfusGjgKSh1yyCV4Hntq1W8PK9LqugyTbWcN54dMVdJR3evXApYbXRxYi58r3TocmQWVbpRhGaLZD62oYcSTVH8paVLVaKTEghm4Xzgss9LZ1rYJVRL3PoisZiN6PNFs573qF1ukuCxqcHjkZqBjdjsapb6ww3uTPVgBK4TBtQ533zHxwc7nJAChKDzwCwMDXMRMjpFSpNPaAq6BUV4fSSp31on2Rj114cVnDys44oVsQxPU1q3xkkshiPxsKxAdqUeu5CpT3pb49WMxZnfoKbbMDRCMaUuyjfforXd8EeDNnoEW9tZq3KLZgecygchi1uj51cQSPps3thF9bUgbaHj334384DHgi5b1L8Lm8F43Y2ugj1Q6jVkyzuKQd1",
    "pool_deposits_address": "zLSQDVBaJ9PZLozVZWfcKd8tBBtriv11j3276DL5LdzpwkJRnPmTBr4KHXrk11cevirazuRwngQeGws2HdMNCDagnqcngybNfDZgmg7Dpa4qjzpQAZgv2CiybkiKf8gbmagfWVcamdVSGCBw9ByHvLrAmARa3Hf28xpGvsRGJur2aWoHs2mpHXpqzYyijKbUsFzUM6uY7ipPpMKjkZBpJ6MYe27bUjP1z4NhBjHvY6Z4T35SPS",
    "datapoint_address": "jL2aaqw6XU61SZznxeykLpREPzSmZv8bwbjEsJD6DMfXQLgBc12wMmPpVD81JnLvRbMphA3SehsyWc4kQ88uKa9SVA3EikNeTUGGQquabVkR4rvvbHgczZPtLhkrmsfE1yLuLFtwBUwuvuEAS4fHHt5ygRC5g3VbsNBhd5oqZGZgmhjgk1zUWLQy6V8zs4K3RxEuEdFWQ58JSBQu8EaR4TnUeAnGyG8Atapku6woNAAUKmT8Vtg6ikEauDY5m",
    "oracle_payout_price": 2000000,
    "live_epoch_length": 5,
    "epoch_prep_length": 5,
    "margin_of_error": 0.01,
    "number_of_oracles": 4,
    "oracle_pool_nft_id": "b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc",
    "oracle_pool_participant_token_id": "12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed"
}
```

#### /nodeInfo
Returns json with information about the node that the oracle is using:
- Node url

Example Response:
```json
{
    "node_url": "http://0.0.0.0:9053"
}
```

#### /poolStatus
Returns the current status of the oracle pool.
- Funded Percentage (Total Funds/Pool Price * 100)
- Current Pool Stage (Epoch Preparation Vs. Live Epoch)
- Latest Pool Datapoint
- Latest Pool Epoch ID
- Height The Next Epoch Ends

Example Response:
```json
{
    "funded_percentage": 1600,
    "current_pool_stage": "Epoch Preparation",
    "latest_pool_datapoint": 251821000,
    "latest_pool_epoch": "14e10314b0b33f13667871c62b0e86904cb6aee854630af4296b567b18875185",
    "next_epoch_ends": 288699
}
```


#### /oracleStatus
Returns the current status of one's own oracle.
- Waiting For Datapoint To Be Submitted To The Core For The Current Epoch (True/False)
- Latest Datapoint Oracle Posted
- Epoch ID That the Datapoint Was Submit In
- Creation Height Of The Datapoint Tx

Example Response:
```json
{
    "waiting_for_datapoint_submit": true,
    "latest_datapoint": 251821000,
    "latest_datapoint_epoch": "14e10314b0b33f13667871c62b0e86904cb6aee854630af4296b567b18875185",
    "latest_datapoint_creation_height": 288677
}
```


### POST API

#### /submitDatapoint
This endpoint allows you to submit a Datapoint during the `Live Epoch` stage.

Example POST JSON Body:
```json
{
    datapoint: 123456
}
```

Example Response:
```json
{
    tx_id: "0d742ecb0d3ffc9cf3104d3da89cf758d200e10e9c4889284c22ea659bcefcc4"
}
```

Allows the owner of an oracle to commit a datapoint for the current running epoch. If the pool is in the epoch preparation stage, the datapoint will be rejected. The provided datapoint must be a valid integer.


