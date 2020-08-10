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

We need to generate two tokens as follows: 
1. A singleton *pool token* to be stored in the bootstrapped epoch preparation box
2. N *oracle tokens* to store in each of the N datapoint boxes. In this case N = `4`.

Step 1. We need the address of the wallet where we want to store the tokens. It should be an address under the pool manager's control. 
This can be an address of the same node where the following request will be made. 
For this example, we will use the address `9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk`. 
 
Step 2. *Generate pool token:* Make a call to the endpoint `/wallet/transaction/send` with the following data:

    {
      "requests": [
        {
          "address": "9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk",
          "amount": 1,
          "name": "POOL",
          "description": "Pool token",
          "decimals": 0
        }
      ],
      "fee": 2000000,
      "inputsRaw": [],
      "dataInputsRaw": []
    }  

After this step, a new token will be generated and sent to the address.  

Step 3. *Generate oracle token:* Make a call to the endpoint `/wallet/transaction/send` with the following data:

    {
      "requests": [
        {
          "address": "9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk",
          "amount": 4,
          "name": "ORA",
          "description": "Oracle token",
          "decimals": 0
        }
      ],
      "fee": 2000000,
      "inputsRaw": [],
      "dataInputsRaw": []
    }  

After generating the tokens add their IDs to the constants defined earlier. 
For the purpose of demonstration, we will use the following token IDs

* pool token ID = `b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc`
* oracle token ID = `12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed`

[TODO describe token generation]

## 3. Compile the contracts

This step explains how to compile the contracts and obtain the corresponding addresses for each contract. 
Since this step is quite large, we will split it into multiple stages.

### 3.1. Compile the Live Epoch Script 

Step 1. Convert the oracle token Id from Hex to Base58. In our case this is:

* oracle token ID = `2GMaYqCG3Hq2xMwiDiA2PuYx2UVLB2EijVrsCMoEyj6L`

Step 2. Substitute the following constants in the live epoch script (given in [`live_epoch.es`](live_epoch.es)):

1. oracleTokenId = Base58 encoded token id = `fromBase58("2GMaYqCG3Hq2xMwiDiA2PuYx2UVLB2EijVrsCMoEyj6L")`
2. epochPeriod = livePeriod + prepPeriod = `10` (blocks)
3. oracleReward = 0.002 Erg = `2000000` (nanoErgs)

The resulting script is below:


    { // LIVE EPOCH SCRIPT
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
        OUTPUTS(0).value >= SELF.value - (oracleBoxes.size + 1) * 2000000 &&
        oracleRewardOutputs._2
      ) && pubKey
    }

Step 3. Compile the above script using a node or the [ErgoScript Playground](https://wallet.plutomonkey.com/p2s/).

The resulting address, given below, is the P2S address of the Live Epoch script (split into multiple lines for readability)

    3vThpSDoLo58CtKKFLBQMmtcD5e5pJeFNNyPKnDRC4zKzhgySeTUkU71fk9mcFgHe23k1b4QuERNdcignnexc\
    ULMEenifBffiNeCdiTkgaUiGtH5D9rrsj698mRLDhANmybx8c6NunwUMoKuLsRoEYtYi8rRjuKfbNDN1HfVsg\
    FKSyKMSnwJXa5KAuABSz5dYUgURf6M3i2bxsKKYTe4uQFEoVcbBwvfW4UxXaKqQYGB8xGLASMfHtcs9R5CBFk\
    HyUSXh2sFy17pfdQ5emx8CgE5ZXRqx7YBYzk9jSyGqp2myT5XvBAS2uSeahNKWYKzh1XTqDc3YGLvBPHJ98bk\
    saaSnNX4SwAhia2mXY4iCKsYf6F7p5QPNjYBXqLyzkDFxSzgQJmMg1Ybh3fx6Sg8esE9w5L7KCGEuydPkBE
    
### 3.2. Compile the Epoch Preparation Script
    
Step 1. Obtain the hex-encoded bytes of the ErgoTree (the language the Ergo node understands) for the Live Epoch Script address 
using the `/script/addressToTree` endpoint of a running node, 
for instance [the one used by the block explorer](http://88.198.13.202:9053/swagger#/script/addressToTree).
For the address above, this comes out to be:

    100d040004000e2012caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed0400\
    050004140402048092f401040201010402058092f4010400d803d601b2a5730000d602b5db6501fed901\
    0263ed93e4c67202050ec5a7938cb2db63087202730100017302d603b17202ea02d1edededededed93cb\
    c27201e4c6a7060e917203730393db63087201db6308a793e4c6720104059db072027304d9010441639a\
    8c720401e4c68c72040206057e72030593e4c6720105049ae4c6a70504730592c1720199c1a77e9c9a72\
    0373067307058cb07202860273087309d901043c400163d802d6068c720401d6078c72060186029a7207\
    730aeded8c72060293c2b2a5720700d0cde4c68c720402040792c1b2a5720700730b02b2ad7202d90104\
    63cde4c672040407730c00

Step 2. Use any tool to compute the blake2b256 hash of the above binary value. The hash (hex-encoded) should come out to be:

    955fd2c22393aa0f5db841dd8a3ad44ebb7de970419f5a0a58441ebe6b809fb2
   
Step 3. Use any tool to convert the above hex-encoded bytes to Base58, which should come out to be:
    
    B46VdKnYx7SyEjh8Z3tDGWkdzRBMD417EqrgbBN88vAD    

Step 4. Substitute the following constants in the epoch preparation script (given in [`epoch_prep.es`](epoch_prep.es)):
        
1. livePeriod = `5` (blocks)
2. buffer = `4` (blocks)
3. epochPeriod = livePeriod + prepPeriod = `10` (blocks)
4. minPoolBoxValue = 0.01 = `10000000` (nanoErgs)
5. liveEpochScriptHash = Base58 encoded live epoch script hash = `fromBase58("B46VdKnYx7SyEjh8Z3tDGWkdzRBMD417EqrgbBN88vAD")`

The resulting script is below:
        
    { // EPOCH PREPARATION SCRIPT
      val canStartEpoch = HEIGHT > SELF.R5[Int].get - 5
      val epochNotOver = HEIGHT < SELF.R5[Int].get
      val epochOver = HEIGHT >= SELF.R5[Int].get
      val enoughFunds = SELF.value >= 10000000
    
      val maxNewEpochHeight = HEIGHT + 10 + 4
      val minNewEpochHeight = HEIGHT + 10
    
      if (OUTPUTS(0).R6[Coll[Byte]].isDefined) {
        val isliveEpochOutput = OUTPUTS(0).R6[Coll[Byte]].get == blake2b256(SELF.propositionBytes) &&
                                blake2b256(OUTPUTS(0).propositionBytes) == fromBase58("B46VdKnYx7SyEjh8Z3tDGWkdzRBMD417EqrgbBN88vAD")
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


    Gxd4hMRT6J1SA6D3tfvyij49J2DCQkeZfxNVEpoZidZtS9YYsi8Jg5u3JBZQHxdmrLpVgTsnLnSbt377BRJA
    WFUfkdcmC1pMPFNUYBWuYaccbMxP5kV3WkGU7oxsWJauKfiGkFZPN1W1RmWVmpFbdKaCizjnMqC7TLsQ53Jf
    BzWo5CsYj2Vn3YYbJFZiXbfVXWKjvkUHatcGxL47QnBffcKfFJun7t1tFgxowLonpFpq7SFAz4YRE6TdZarm
    WDjDER13pSUupfaKCZmUe3aCRhgAsdp4RHuW8n1RywcYcSjGNPVFzsGjD8GQdUrs85Xv4gobuH49S4WZFgkc
    oQAx3jx3GqhY9kQWwdn7Ni7v2XcKMwFFCvvzrPAKtUHLZYU4VN4RjvoFLRYJ5H
        
### 3.3. Compile the Data Point Script

Step 1. Convert the pool token ID from Hex to Base58. In our case this is are:

* pool token ID = `DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF`

Step 2. Substitute the following constants in the data point script (given in [`data_point.es`](data_point.es)):

1. poolTokenId = Base58 encoded token id = `fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF")`
2. liveEpochScriptHash = Base58 encoded live epoch script hash = `fromBase58("B46VdKnYx7SyEjh8Z3tDGWkdzRBMD417EqrgbBN88vAD")`

The resulting script is below:

    {
     // This box:
     // R4: The address of the oracle (never allowed to change after bootstrap).
     // R5: The box id of the latest Live Epoch box.
     // R6: The oracle's datapoint.
    
     val pubKey = SELF.R4[GroupElement].get
    
     val liveEpochBox = CONTEXT.dataInputs(0)
    
     val validLiveEpochBox = liveEpochBox.tokens(0)._1 == fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF") &&
    			 blake2b256(liveEpochBox.propositionBytes) == fromBase58("B46VdKnYx7SyEjh8Z3tDGWkdzRBMD417EqrgbBN88vAD")
    
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

    jL2aaqw6XU61SZznxeykLpREPzSmZv8bwbjEsJD6DMfXQLgBc12wMmPpVD81JnLxfxkT6s5nvYgbB62vkH8C\
    hHeuVKtCPDMLTZ3gFMTa11YXXGBKvkezBENzpDBh8HsLHhnTTbMzv2sViDQpSWVNEF6G3Z9Fn2Ce6TNc5iHF\
    Zr7jGCBLtfRLKMb9RRUc9voWz9yEWpgADEkoQnDyMn5wc6xLoJsSYLfXHo2t8pyvwXfn2NotR3xFRDHU7wHXe
    
### 3.4. Compile the Pool Deposit Script

Step 1. Obtain the hex-encoded bytes of the ErgoTree (the language the Ergo node understands) for the Epoch Preparation Script address 
using the `/script/addressToTree` endpoint of a running node, 
for instance [the one used by the block explorer](http://88.198.13.202:9053/swagger#/script/addressToTree).
For the address above, this comes out to be:

    100604000580dac40904140e20955fd2c22393aa0f5db841dd8a3ad44ebb7de970419f5a0a58441ebe6b\
    809fb2040a0408d806d601b2a5730000d602c67201060ed603e4c6a70504d604c1a7d6059272047301d6\
    069aa3730295e67202d801d607ed93e47202cbc2a793cbc272017303eb02d1ededededededed8fa37203\
    91a39972037304720593e4c672010405e4c6a7040593e4c672010504720393db63087201db6308a792c1\
    720172047207d1ededededededed92a37203720593e4c672010405e4c6a7040592e4c672010504720690\
    e4c6720105049a7206730593db63087201db6308a792c1720172047207d1edededed93e4c672010405e4\
    c6a7040593e4c672010504720393c27201c2a793db63087201db6308a791c172017204

Step 2. Use any tool to compute the blake2b256 hash of the above binary value. The hash (hex-encoded) should come out to be:

    5ea046c8753cbf8bb0acdbd67dd8a5d905df89d67060624282ad757fa3cb670c
   
Step 3. Use any tool to convert the above hex-encoded bytes to Base58, which should come out to be:
    
    7NP5EMZUh6KbZSuY21oyvfvP3Bg8wKLHRVpyjZM7QkoV

Step 4. Substitute the following constants in the pool deposit script (given in [`pool_deposit.es`](pool_deposit.es)):
        
1. poolTokenId = Base58 encoded token id = `fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF")`
2. epochPrepScriptHash = Base58 encoded live epoch script hash = `fromBase58("7NP5EMZUh6KbZSuY21oyvfvP3Bg8wKLHRVpyjZM7QkoV")`

The resulting script is below:

    {
      val allFundingBoxes = INPUTS.filter{(b:Box) =>
        b.propositionBytes == SELF.propositionBytes
      }
    
      val totalFunds = allFundingBoxes.fold(0L, { (t:Long, b: Box) => t + b.value })
    
      sigmaProp(
        blake2b256(INPUTS(0).propositionBytes) == fromBase58("7NP5EMZUh6KbZSuY21oyvfvP3Bg8wKLHRVpyjZM7QkoV") &&
        OUTPUTS(0).propositionBytes == INPUTS(0).propositionBytes &&
        OUTPUTS(0).value >= INPUTS(0).value + totalFunds &&
        OUTPUTS(0).tokens(0)._1 == fromBase58("DGxdZaamaCttUuTNDVcxXw1i5N4Aom5Hj4uFKSC7B4nF")
      )
    }
        
    
Step 3. Compile the above script using a node or the [ErgoScript Playground](https://wallet.plutomonkey.com/p2s/).
        
The resulting address, given below, is the P2S address of the Data Point script (split into multiple lines for readability)

    zLSQDVBaFJVVPWsvzN8begiciWsjdiFyJn9NwnLbJxMrGehDXPJnEuWm2x8gQtCutoK7crMSP9sKQBPyaPVR\
    QXpiSr7ZoKrz4arYiJXKX1MDAfJFm9tjkY379ZiskLYHC3mmf4CQxATbY9P3mTjYw3f3Hkoxnu4yxvMCVBtR\
    TuuRK1qh4E6aGpG8cJcpJ5qBtEsx7SrJoMZP34exMNxD1dPoaDFbuKHnoXAZmDLHnLqG3HgdPy
      
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
          "address": "Gxd4hMRT6J1SA6D3tfvyij49J2DCQkeZfxNVEpoZidZtS9YYsi8Jg5u3JBZQHxdmrLpVgTsnLnSbt377BRJAWFUfkdcmC1pMPFNUYBWuYaccbMxP5kV3WkGU7oxsWJauKfiGkFZPN1W1RmWVmpFbdKaCizjnMqC7TLsQ53JfBzWo5CsYj2Vn3YYbJFZiXbfVXWKjvkUHatcGxL47QnBffcKfFJun7t1tFgxowLonpFpq7SFAz4YRE6TdZarmWDjDER13pSUupfaKCZmUe3aCRhgAsdp4RHuW8n1RywcYcSjGNPVFzsGjD8GQdUrs85Xv4gobuH49S4WZFgkcoQAx3jx3GqhY9kQWwdn7Ni7v2XcKMwFFCvvzrPAKtUHLZYU4VN4RjvoFLRYJ5H",
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
          "address": "jL2aaqw6XU61SZznxeykLpREPzSmZv8bwbjEsJD6DMfXQLgBc12wMmPpVD81JnLxfxkT6s5nvYgbB62vkH8ChHeuVKtCPDMLTZ3gFMTa11YXXGBKvkezBENzpDBh8HsLHhnTTbMzv2sViDQpSWVNEF6G3Z9Fn2Ce6TNc5iHFZr7jGCBLtfRLKMb9RRUc9voWz9yEWpgADEkoQnDyMn5wc6xLoJsSYLfXHo2t8pyvwXfn2NotR3xFRDHU7wHXe",
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
    
    
    
