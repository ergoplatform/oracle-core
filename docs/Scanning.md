### UTXO Set Scanning
Using the new [EIP-1](https://github.com/ergoplatform/eips/blob/master/eip-0001.md), the oracle core will register scans to find all relevant boxes.

The initial implementation includes scans for:

1. A box at the `Datapoint` contract address which also contains the oracle's address in R4 and the oracle pool participant token.
2. Boxes at the `Datapoint` contract address and the oracle pool participant token.
3. Any boxes at the `Pool Deposit` contract address.
4. A box at the "Epoch Preparation" contract address which holds the oracle pool NFT.
5. A box at the "Oracle Pool Epoch" contract address which holds the oracle pool NFT.

The oracle core saves each of the `scanId`s locally into `scanIDs.json` after registering them with the full node. At any time the oracle core wishes to check the current state of the protocol, it simply reads the `scanId`s and acquires all of the relevant unspent boxes from the node.

