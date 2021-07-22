# Bootstrap CLI V0.1
This is a CLI tool which simplifies the process of bootstrapping an oracle pool. The CLI tool exposes 5 actions, one for each of the following tasks:
1. `generatePoolToken` to generate a singleton pool token
2. `generateOracleTokens` to generate multiple oracle tokens
3. `getContractAddresses` Getting the addresses using the tokens generated
4. `bootStrapPool` Bootstrapping the epoch preparation box using the address and tokens
5. `bootStrapOracle` Bootstrapping the oracle boxes using the address and tokens

## Step 1: Configure options

Create a file called `application.conf` (or download the [example config here](application.conf)). Here is an example config for a pool:

    ergo.oraclepool {
        pool {
            oracleReward = 2500000
            minBoxValue = 1500000
            maxOutlierPercent = 50
            numOracles = 14
            postingSchedule = 30
            liveEpochPeriod = 20
            buffer = 5
        }
        node {
            apiKey = "hello"
            baseUrl = "http://192.168.0.200:9053/"
            defaultFee = 3000000
        }
    }

Edit the file with the desired parameters for your oracle pool.

## Step 2a: Compile The Jar (Optional)

Skip this step if using the pre-compiled jar.

Use the command:

    sbt assembly

The jar will be stored as `target/scala-2.12/oracle-pool-bootstrap.jar`.
Copy the jar to some desired location.


## Step 2b: Copy The Pre-Compiled Jar

Alternatively a pre-compiled `oracle-pool-bootstrap.jar` is provided in this folder which can be downloaded and used without having to have the JDK/sbt installed or doing the compiling yourself.


## Step 3: Running The Bootstrap CLI Tool

Running the jar is very simple and provides us with basic usage information:

    java -jar oracle-pool-bootstrap.jar

    Usage:
    java -cp <jarFile> generatePoolToken <recipientAddress>
    java -cp <jarFile> generateOracleToken <recipientAddress> <numOracles>
    java -cp <jarFile> getContractAddresses <oracleTokenId> <poolTokenId>
    java -cp <jarFile> bootStrapPool <oracleTokenId> <poolTokenId> <initialDataPoint_serialized>
    java -cp <jarFile> bootStrapOracle <oracleTokenId> <poolTokenId> <rewardAddress>


## Step 4: Issue tokens

Usage:

    java -cp oracle-pool-bootstrap.jar generatePoolToken <address_to_store_tokens_in>
    java -cp oracle-pool-bootstrap.jar generateOracleTokens <address_to_store_tokens_in> <num_tokens>

Example:

    java -cp oracle-pool-bootstrap.jar generatePoolToken 9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk
    java -cp oracle-pool-bootstrap.jar generateOracleTokens 9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk 20

## Step 5: Get Smart Contract Addresses

Usage:

    java -cp oracle-pool-bootstrap.jar getContractAddresses <oracleTokenId> <poolTokenId>

Example:

    java -cp oracle-pool-bootstrap.jar getContractAddresses 12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc

Output:

    LiveEpoch: USNVj4rN2DctyA1X4Wc7k1DpgW8MgSmLbXxeDgL3CvheUhjTRZPB4Pp6tqG8yZnLQkJVEvgHeWQjMwbKcYdKkGwKYN7coUCPXDwVjsniPEu7wuyzVsWrDH4q8CRatjfiVE3U8growjbfNUq6xcg8AQdShGAhduZpYeUULp7bgTHwQe7c1oWaFLKszSaZwKExY8TtrvJJacK4mj5bMFAzYGwrTNvMenpYqaiUfPd5e5i2vx3dT23RXYpJs6GZ4Mgyr2yYo98MKWUhxnfBK4UBSm1MLwH3p3oFii3L2KkUrZpBtP5tckGHVi98Hwew2cMhWNugyVRv328MSXTm8USZx1DpAvRxDd4JgCjnxzfWyFUa1qvWXKDcpig9Q9WMucXn8USd1vjT5n5V4h5kBxqsNFZzRmbTGA7KUmgnTu7kFE5PjkfJZSPNZJNmazG6UmjBZhs6DXnaHTnccFtjC11eWYsMm1pU4d4Y4fsJpd6z
    EpochPrep: Gxd4hMRT5aE7dxAoMpoGtj74HorsenAQqMhxvzpT7GQEbeUTrx3ZjYHotQn54Ye1ddrLLmLsya6ryMvJYPkcdkCvyv6CMBshCDpEebRdfKGxKaNZ3QEYPomT1eqX594zZJhXddD9eGSXndCzSWLVkcxpFx3ubCWo7zCox1hZqWMbSUGmXLJPWRLpGb8DTBTzxEfqSugnJEUwgg7a2hyh61wthK4FCM7y3zk4vNYYrdGWwKAW2Dz5VPoHaMh3zbRtQbwpdfYBdSbbBHn4MexaXe9SHNU9aP5mSzb8cnYGgE22kLgtYeBq3BPmqeJp3usRz3QYuCE8Z727n6fFGHzJw5drVWetG24eqYkzVoN7mF6DtRxsjrXnQF3u3ofzgzKPgR7Hi6Me8Puz2s
    DataPoint: jL2aaqw6XU61SZznxeykLpREPzSmZv8bwbjEsJD6DMfXQLgBc12wMmPpVD81JnLuZUjHRKPysKxKQhcBaqDs7ZAtYwRuYmQojzKK9bHXDUY8N4BiJx8AUG8VEaggD4ztWSeQHrW7EbFxpXgaMKuzuN1Gq4zoYDArstgcrHKwg2uCeGeXiydQXRWEyE8e6noAP13nUBSmNNNVqkM9JGUVAJYo4GGdVFg8FRtFWcNdtbxCKfw4JGVhakCGj4qvd
    Deposit: zLSQDVBaDkFiQhpgVYqu9saX3ppCMzmma1qdryGH1x1GTkAjU9vVodDsYrk3H5UvqDmdxJLoDADg69KXyL9gVGW2NER7GxMotdh46Bzr9P9tJwPdgvNhSdoXYrLTemKadCU46aGy81YneoKB7xjz3a1v4Aar3n71XysQ6HwdKcJt8WFKqbZmRx4JnJTtBUtsdD184oU623BXA93cGrG1fFuFzSALqGztnS9Ai4JP6NcM8LE2wU

## Step 6: Bootstrap Pool

In order to bootstrap the pool we need a recent datapoint close to the value that will be committed by oracles (to ensure that they lie within the outlier range of 50%).
The data-point is a long value and must be serialized properly for storing in the register.
Details of the serialization are available in the [ErgoTree specification (Section 5.2)](https://ergoplatform.org/docs/ErgoTree.pdf).

Every oracle pool needs to select it's own oracle core connector. This is the piece of software which fetches your oracle pool's datapoint from the outside world.
You can find a list of connectors [in this folder](../connectors), and instructions to [compile the oracle core/connector of your choice here](https://github.com/ergoplatform/oracle-core).

Once you have compiled your selected connector, you can now generate a recent datapoint which will be used for bootstrapping your pool.
The `--bootstrap-value` command can be used with any connector to do so, as such:

    ./erg-usd-connector --bootstrap-value

This will print out some value such as `0502`. Copy that value and invoke the CLI as:

    java -cp oracle-pool-bootstrap.jar bootstrapPool <oracleTokenId> <poolTokenId> <initial_datapoint>

Example:

    java -cp oracle-pool-bootstrap.jar bootstrapPool 12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc 0502

This will create the Epoch Preparation box with the desired token.

## Step 7: Bootstrap Oracles

To bootstrap the Datapoint boxes for the oracles, do the following for each oracle. First obtain the oracle's full node wallet address which they will be using to take part in the oracle pool.
This address must be an "ordinary" P2PK address (any address generated by the node's wallet will do). We call this the `oracleAddress`.

For the technically inclined, an ordinary address corresponds to the script `proveDlog(h)` for some elliptic curve point `h` on the Secp256k1 curve, which is
the same curve used in Bitcoin.

An example of a ordinary address is `9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk`. To bootstrap the oracle issue the following command.

    java -cp oracle-pool-bootstrap.jar ergo.oraclepool.BootStrapOracle <oracleTokenId> <poolTokenId> <oracleAddress>

Example:

    java -cp oracle-pool-bootstrap.jar ergo.oraclepool.BootStrapOracle 12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed b662db51cf2dc39f110a021c2a31c74f0a1a18ffffbf73e8a051a7b8c0f09ebc 9fcrXXaJgrGKC8iu98Y2spstDDxNccXSR9QjbfTvtuv7vJ3NQLk

This will create the Datapoint box with the desired token.
