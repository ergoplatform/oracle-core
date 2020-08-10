# Bootstrapping the oracle pool

This article explains how to bootstrap the oracle pool. In summary, the following steps will be performed. 

1. Finalize constants such as epoch length, total number of oracles, etc.
2. Generate the tokens 
3. Compile the contracts
4. Bootstrap the epoch-prep box
5. Bootstrap the datapoint boxes for every oracle

You will need to have a full node running and synced for steps 2 to 4.  

## 1. Finalize constants

For the demonstration, we will define the following constants:
1. Live epoch length (livePeriod) = 5 blocks
2. Epoch prep length (prepPeriod) = 5 blocks
3. Buffer for delay (buffer) = 4 blocks
4. Oracle reward per datapoint (oracleReward) = 0.002 Erg
5. Total number of oracles (N) = 4

## 2. Generate the tokens

Generate two tokens as follows: 
1. A singleton *pool token* to be stored in the bootstrapped epoch preparation box
2. N *oracle tokens* to store in each of the N datapoint boxes.

After generating the tokens add their IDs to the constants defined earlier. 
For the purpose of demonstration, we will use the following token IDs

* pool token ID = b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc
* oracle token ID = 12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed

[TODO describe token generation]

## 3. Compile the contracts

This step explains how to compile the contracts and obtain the corresponding addresses for each contract. 
Since this step is quite large, we will split it into multiple stages.

### 3.1. Compile the Live Epoch Script 

Step 1. Convert the oracle token Id from Hex to Base58. In our case this is:

* oracle token ID = 2GMaYqCG3Hq2xMwiDiA2PuYx2UVLB2EijVrsCMoEyj6L

Step 2. Substitute the following constants in the live epoch script (given in [`live_epoch.es`](live_epoch.es)):

1. oracleTokenId = Base58 encoded token id = `fromBase58("2GMaYqCG3Hq2xMwiDiA2PuYx2UVLB2EijVrsCMoEyj6L")`
2. epochPeriod = livePeriod + prepPeriod = `10` (blocks)
3. oracleReward = 0.002 Erg = `2000000` (nanoErgs)
4. minPoolBoxValue = 0.01 = `10000000` (nanoErgs)

The resulting script is below:


    {
      val oracleBoxes = CONTEXT.dataInputs.filter{(b:Box) =>
        b.R5[Coll[Byte]].get == SELF.id &&
        b.tokens(0)._1 == fromBase58("2GMaYqCG3Hq2xMwiDiA2PuYx2UVLB2EijVrsCMoEyj6L")
      }
    
      val pubKey = oracleBoxes.map{(b:Box) => proveDlog(b.R4[GroupElement].get)}(0)
    
      val sum = oracleBoxes.fold(0L, { (t:Long, b: Box) => t + b.R6[Long].get })
    
      val average = sum / oracleBoxes.size // do we need to check for division by zero here?
    
      val oracleRewardOutputs = oracleBoxes.fold((1, true), { (t:(Int, Boolean), b:Box) =>
        (t._1 + 1, t._2 &&
                   OUTPUTS(t._1).propositionBytes == proveDlog(b.R4[GroupElement].get).propBytes &&
                   OUTPUTS(t._1).value >= 2000000)
      })
    
      val epochPrepScriptHash = SELF.R6[Coll[Byte]].get
    
      sigmaProp(
        blake2b256(OUTPUTS(0).propositionBytes) == epochPrepScriptHash &&
        oracleBoxes.size > 0 &&
        OUTPUTS(0).tokens == SELF.tokens &&
        OUTPUTS(0).R4[Long].get == average &&
        OUTPUTS(0).R5[Int].get == SELF.R5[Int].get + 10 &&
        OUTPUTS(0).value >= 10000000 &&
        OUTPUTS(0).value >= SELF.value - (oracleBoxes.size + 1) * 2000000 &&
        oracleRewardOutputs._2
      ) && pubKey
    }

Step 3. Compile the above script using a node or the [ErgoScript Playground](https://wallet.plutomonkey.com/p2s/).

The resulting address, given below, is the P2S address of the Live Epoch script (split into multiple lines for readability)


    F7f7vfC28mjet2UuH3cKyL7CHbtnc1AgxnEBQ6UhMvRtxhX7BrG7MLhMj7JEcmMyQqRzHg7hfoLNSzoDWg4P\
    WfqSxoZXkTBPUWharJCtoRjaoHGYgjF9BJCjDNR13EwMVoXBhY2gmgfWyCjKjncFpjzbSBQYRAsj7W5vg3A2\
    NtXudGMn2YjfHSqjFk1xzV4sfYGtfM9fLfd3ZEBMFfQpPRapG4DXaGL8emVrRsqfjGDqVRkxw1kJyffbFTsD\
    StRgrKeGbA1gZKsKAYJiWLYVbmndRxhUuM7fQhtX8qzRMpDfqti43eotgxVXU5pr9Q7a4Pv2VbvS8gBDceRP\
    ZeLdsxBiDoWVbGEkF8vB7QrDNr9YxXEob4KircTpECARmcGgeLCHwr2i7AMGbs2tFFLX7PoHyYRv3ertFGS1\
    CEth6wnjmo3SEjK8HXU

### 3.2. Compile the Epoch Preparation Script
    
Step 1. Obtain the hex-encoded bytes of the ErgoTree (the language the Ergo node understands) for the Live Epoch Script address 
using the `/script/addressToTree` endpoint of a running node, 
for instance [the one used by the block explorer](http://88.198.13.202:9053/swagger#/script/addressToTree).
For the address above, this comes out to be:

    100e040004000e2012caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed0400\
    050004140580dac4090402048092f401040201010402058092f4010400d803d601b2a5730000d602b5db\
    6501fed9010263ed93e4c67202050ec5a7938cb2db63087202730100017302d603b17202ea02d1ededed\
    edededed93cbc27201e4c6a7060e917203730393db63087201db6308a793e4c6720104059db072027304\
    d9010441639a8c720401e4c68c72040206057e72030593e4c6720105049ae4c6a70504730592c1720173\
    0692c1720199c1a77e9c9a720373077308058cb0720286027309730ad901043c400163d802d6068c7204\
    01d6078c72060186029a7207730beded8c72060293c2b2a5720700d0cde4c68c720402040792c1b2a572\
    0700730c02b2ad7202d9010463cde4c672040407730d00

Step 2. Use any tool to compute the blake2b256 hash of the above binary value. The hash (hex-encoded) should come out to be:

    814e3669af07fde7757fbf063131b2ffea3b9534273e6dac0b1830373e81f079
   
Step 3. Use any tool to convert the above hex-encoded bytes to Base58, which should come out to be:
    
    9hkmM56mTUHKJdfU5fvVTUHjnVxX67etUc7n32mU5znt

Step 4. Substitute the following constants in the epoch preparation script (given in [`epoch_prep.es`](epoch_prep.es)):
        
1. livePeriod = `5` (blocks)
2. buffer = `4` (blocks)
3. epochPeriod = livePeriod + prepPeriod = `10` (blocks)
4. minPoolBoxValue = 0.01 = `10000000` (nanoErgs)
5. liveEpochScriptHash = Base58 encoded live epoch script hash = `fromBase58("9hkmM56mTUHKJdfU5fvVTUHjnVxX67etUc7n32mU5znt")`

The resulting script is below:
        
    {
      val canStartEpoch = HEIGHT > SELF.R5[Int].get - 5
      val epochNotOver = HEIGHT < SELF.R5[Int].get
      val epochOver = HEIGHT >= SELF.R5[Int].get
      val enoughFunds = SELF.value >= 10000000
    
      val maxNewEpochHeight = HEIGHT + 10 + 4
      val minNewEpochHeight = HEIGHT + 10
    
      if (OUTPUTS(0).R6[Coll[Byte]].isDefined) {
        val isliveEpochOutput = OUTPUTS(0).R6[Coll[Byte]].get == blake2b256(SELF.propositionBytes) &&
                                blake2b256(OUTPUTS(0).propositionBytes) == fromBase58("9hkmM56mTUHKJdfU5fvVTUHjnVxX67etUc7n32mU5znt")
        sigmaProp( // start next epoch
          epochNotOver && canStartEpoch && enoughFunds &&
          OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
          OUTPUTS(0).R5[Int].get == SELF.R5[Int].get &&
          OUTPUTS(0).tokens == SELF.tokens &&
          OUTPUTS(0).value >= SELF.value &&
          isliveEpochOutput
        ) || sigmaProp( // create new epoch
          epochOver &&
          enoughFunds &&
          OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
          OUTPUTS(0).R5[Int].get >= minNewEpochHeight &&
          OUTPUTS(0).R5[Int].get <= maxNewEpochHeight &&
          OUTPUTS(0).tokens == SELF.tokens &&
          OUTPUTS(0).value >= SELF.value &&
          isliveEpochOutput
        )
      } else {
        sigmaProp( // collect funds
          OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
          OUTPUTS(0).R5[Int].get == SELF.R5[Int].get &&
          OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
          OUTPUTS(0).tokens == SELF.tokens &&
          OUTPUTS(0).value > SELF.value
        )
      }
    }

Step 5. Compile the above script using a node or the [ErgoScript Playground](https://wallet.plutomonkey.com/p2s/).

The resulting address, given below, is the P2S address of the Epoch Preparation script (split into multiple lines for readability)


    Gxd4hMRT6J1SA6D3tfusGjgKSh1yyCV4Hntq1W8PK9LqugyTbWcN54dMVdJR3evXApYbXRxYi58r3TocmQWV
    bpRhGaLZD62oYcSTVH8paVLVaKTEghm4Xzgss9LZ1rYJVRL3PoisZiN6PNFs573qF1ukuCxqcHjkZqBjdjsa
    pb6ww3uTPVgBK4TBtQ533zHxwc7nJAChKDzwCwMDXMRMjpFSpNPaAq6BUV4fSSp31on2Rj114cVnDys44oVs
    QxPU1q3xkkshiPxsKxAdqUeu5CpT3pb49WMxZnfoKbbMDRCMaUuyjfforXd8EeDNnoEW9tZq3KLZgecygchi
    1uj51cQSPps3thF9bUgbaHj334384DHgi5b1L8Lm8F43Y2ugj1Q6jVkyzuKQd1
    
### 3.3. Compile the Data Point Script

Step 1. Convert the pool token ID from Hex to Base58. In our case this is are:

* pool token ID = DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF

Step 2. Substitute the following constants in the data point script (given in [`data_point.es`](data_point.es)):

1. poolTokenId = Base58 encoded token id = `fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF")`
2. liveEpochScriptHash = Base58 encoded live epoch script hash = `fromBase58("9hkmM56mTUHKJdfU5fvVTUHjnVxX67etUc7n32mU5znt")`

The resulting script is below:

    {
     // This box:
     // R4: The address of the oracle (never allowed to change after bootstrap).
     // R5: The box id of the latest Live Epoch box.
     // R6: The oracle's datapoint.
    
     val pubKey = SELF.R4[GroupElement].get
    
     val liveEpochBox = CONTEXT.dataInputs(0)
    
     val validLiveEpochBox = liveEpochBox.tokens(0)._1 == fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF") &&
    			 blake2b256(liveEpochBox.propositionBytes) == fromBase58("9hkmM56mTUHKJdfU5fvVTUHjnVxX67etUc7n32mU5znt")
    
     sigmaProp(
       OUTPUTS(0).R4[GroupElement].get == pubKey &&
       OUTPUTS(0).R5[Coll[Byte]].get == liveEpochBox.id &&
       OUTPUTS(0).R6[Long].get > 0 &&
       OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
       OUTPUTS(0).tokens == SELF.tokens &&
       validLiveEpochBox
     ) && proveDlog(pubKey)
    }
    
Step 3. Compile the above script using a node or the [ErgoScript Playground](https://wallet.plutomonkey.com/p2s/).
        
The resulting address, given below, is the P2S address of the Data Point script (split into multiple lines for readability)

    jL2aaqw6XU61SZznxeykLpREPzSmZv8bwbjEsJD6DMfXQLgBc12wMmPpVD81JnLvRbMphA3SehsyWc4kQ88u\
    Ka9SVA3EikNeTUGGQquabVkR4rvvbHgczZPtLhkrmsfE1yLuLFtwBUwuvuEAS4fHHt5ygRC5g3VbsNBhd5oq\
    ZGZgmhjgk1zUWLQy6V8zs4K3RxEuEdFWQ58JSBQu8EaR4TnUeAnGyG8Atapku6woNAAUKmT8Vtg6ikEauDY5m

### 3.4. Compile the Pool Deposit Script

Step 1. Obtain the hex-encoded bytes of the ErgoTree (the language the Ergo node understands) for the Epoch Preparation Script address 
using the `/script/addressToTree` endpoint of a running node, 
for instance [the one used by the block explorer](http://88.198.13.202:9053/swagger#/script/addressToTree).
For the address above, this comes out to be:

    100604000580dac40904140e20814e3669af07fde7757fbf063131b2ffea3b9534273e6dac0b1830373e\
    81f079040a0408d806d601b2a5730000d602c67201060ed603e4c6a70504d604c1a7d6059272047301d6\
    069aa3730295e67202d801d607ed93e47202cbc2a793cbc272017303eb02d1ededededededed8fa37203\
    91a39972037304720593e4c672010405e4c6a7040593e4c672010504720393db63087201db6308a792c1\
    720172047207d1ededededededed92a37203720593e4c672010405e4c6a7040592e4c672010504720690\
    e4c6720105049a7206730593db63087201db6308a792c1720172047207d1edededed93e4c672010405e4\
    c6a7040593e4c672010504720393c27201c2a793db63087201db6308a791c172017204        

Step 2. Use any tool to compute the blake2b256 hash of the above binary value. The hash (hex-encoded) should come out to be:

    b413a0bd1a41798d5ce92e044e4a064e887639fad61e3ee83ba12117465c3659
   
Step 3. Use any tool to convert the above hex-encoded bytes to Base58, which should come out to be:
    
    D7wkBq6HAJQQ27uray2qnji8WpwN6oXdvCH6XzyZAVSQ

Step 4. Substitute the following constants in the pool deposit script (given in [`pool_deposit.es`](pool_deposit.es)):
        
1. poolTokenId = Base58 encoded token id = `fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF")`
2. epochPrepScriptHash = Base58 encoded live epoch script hash = `fromBase58("D7wkBq6HAJQQ27uray2qnji8WpwN6oXdvCH6XzyZAVSQ")`

The resulting script is below:

    {
      val allFundingBoxes = INPUTS.filter{(b:Box) =>
        b.propositionBytes == SELF.propositionBytes
      }
    
      val totalFunds = allFundingBoxes.fold(0L, { (t:Long, b: Box) => t + b.value })
    
      sigmaProp(
        blake2b256(INPUTS(0).propositionBytes) == fromBase58("D7wkBq6HAJQQ27uray2qnji8WpwN6oXdvCH6XzyZAVSQ") &&
        OUTPUTS(0).propositionBytes == INPUTS(0).propositionBytes &&
        OUTPUTS(0).value >= INPUTS(0).value + totalFunds &&
        OUTPUTS(0).tokens(0)._1 == fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF")
      )
    }
        
    
Step 3. Compile the above script using a node or the [ErgoScript Playground](https://wallet.plutomonkey.com/p2s/).
        
The resulting address, given below, is the P2S address of the Data Point script (split into multiple lines for readability)

    zLSQDVBaJ9PZLozVZWfcKd8tBBtriv11j3276DL5LdzpwkJRnPmTBr4KHXrk11cevirazuRwngQeGws2HdMN\
    CDagnqcngybNfDZgmg7Dpa4qjzpQAZgv2CiybkiKf8gbmagfWVcamdVSGCBw9ByHvLrAmARa3Hf28xpGvsRG
    Jur2aWoHs2mpHXpqzYyijKbUsFzUM6uY7ipPpMKjkZBpJ6MYe27bUjP1z4NhBjHvY6Z4T35SPS
      
## 4. Bootstrap the Epoch Preparation Box

In order to bootstrap the epoch preparation box, we need to create a box with the epoch preparation script address with its registers 
populated as follows:
        
* R4: Dummy data point placeholder (Long). We will use 1, which is serialized as `0502` (hex).
* R5: Dummy placeholder epoch end height (Int). It can be any value below the current height. We will use 200000, which is serialized as `0480b518`. 

We should have access to a node with the address having the singleton pool token.  

To bootstrap the pool, call the [`/wallet/transaction/send`](http://88.198.13.202:9053/swagger#/wallet/walletTransactionGenerateAndSend) endpoint of **that** node with the following data: 

    {
      "requests": [
        {
          "address": "Gxd4hMRT6J1SA6D3tfusGjgKSh1yyCV4Hntq1W8PK9LqugyTbWcN54dMVdJR3evXApYbXRxYi58r3TocmQWVbpRhGaLZD62oYcSTVH8paVLVaKTEghm4Xzgss9LZ1rYJVRL3PoisZiN6PNFs573qF1ukuCxqcHjkZqBjdjsapb6ww3uTPVgBK4TBtQ533zHxwc7nJAChKDzwCwMDXMRMjpFSpNPaAq6BUV4fSSp31on2Rj114cVnDys44oVsQxPU1q3xkkshiPxsKxAdqUeu5CpT3pb49WMxZnfoKbbMDRCMaUuyjfforXd8EeDNnoEW9tZq3KLZgecygchi1uj51cQSPps3thF9bUgbaHj334384DHgi5b1L8Lm8F43Y2ugj1Q6jVkyzuKQd1",
          "value": 20000000,
          "assets": [
            {
              "tokenId": "b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc",
              "amount": 1
            }
          ],
          "registers": {
            "R4": "0502",
            "R5": "0480b518"
          }
        }
      ],
      "fee": 2000000,
      "inputsRaw": [],
      "dataInputsRaw": []
    }
    
## 5. Bootstrap the Oracle Data Point Boxes

Step 1. Obtain the group element corresponding to the address of the oracle. In our example, the address is `9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk`.

* Call the [`/utils/addressToRaw`](http://88.198.13.202:9053/swagger#/utils/AddressToRaw) endpoint to obtain the hex encoded elliptic curve point corresponding to the address. 
* This turns out to be `0290d9bbac88042a69660b263b4afc29a2084a0ffce4665de89211846d42bb30e4`. 
* Append `07` (the type) to obtain the final value to be stored in R4 of the data-point box.  

Step 2. In order to bootstrap each oracle data point box, we need to create a box with the data point script address with its registers 
populated as follows:
        
* R4: Public key of the oracle encoded, the value from Step 1, which is serialized as `070290d9bbac88042a69660b263b4afc29a2084a0ffce4665de89211846d42bb30e4`.
* R5: Dummy placeholder live epoch box ID (`Coll[Byte]`). We will use `0x01`, which is serialized as `0e0101`. 
* R6: Dummy placeholder data point (Long). We will use 1, which is serialized as `0502`.

We should have access to a node with the address that stores the initially created **N** oracle tokens.  
To bootstrap the pool, call the [`/wallet/transaction/send`](http://88.198.13.202:9053/swagger#/wallet/walletTransactionGenerateAndSend) endpoint of **that** node with the following data: 

    {
      "requests": [
        {
          "address": "jL2aaqw6XU61SZznxeykLpREPzSmZv8bwbjEsJD6DMfXQLgBc12wMmPpVD81JnLvRbMphA3SehsyWc4kQ88uKa9SVA3EikNeTUGGQquabVkR4rvvbHgczZPtLhkrmsfE1yLuLFtwBUwuvuEAS4fHHt5ygRC5g3VbsNBhd5oqZGZgmhjgk1zUWLQy6V8zs4K3RxEuEdFWQ58JSBQu8EaR4TnUeAnGyG8Atapku6woNAAUKmT8Vtg6ikEauDY5m",
          "value": 2000000,
          "assets": [
            {
              "tokenId": "12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed",
              "amount": 1
            }
          ],
          "registers": {
            "R4": "070290d9bbac88042a69660b263b4afc29a2084a0ffce4665de89211846d42bb30e4",
            "R5": "0e0101",
            "R6": "0502"
          }
        }
      ],
      "fee": 2000000,
      "inputsRaw": [],
      "dataInputsRaw": []
    }
    
    
    
