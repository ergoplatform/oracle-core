# Oracle Core

![](images/oracle-core.png)

The oracle core is the off-chain component that oracles who are part of an oracle pool run. This oracle core provides a HTTP API interface for reading the current protocol state & another for submitting datapoints. Once a datapoint is submited, the oracle core will automatically generate the required tx and post it as well as any other actions required for the protocol to run. This thereby allows the oracle to participate in the oracle pool protocol without any extra effort for the oracle operator.

The oracle core requires that the user has access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools then a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).

A `Connector` must also be used with the oracle core in order to acquire data to submit to the pool. Each connector sources data from the expected sources, potentially applies functions to said data, and then submits the data to the oracle core via HTTP API during the `Live Epoch` stage in the oracle pool protocol. All oracles for a given pool are expected to use the exact same connector, thereby making it simple to onboard and get started.

The current oracle core is built to run the protocol specified in the [Deviation Checking Oracle Pool Spec](/docs/specs/v0.2/Deviation-Checking-Oracle-Pool-Spec.md).

Other documents can also be found explaining how various parts of the oracle core work in the [docs folder](docs).



# Building & Running An Oracle
The majority of oracle operators will only need to focus on setting up their own oracle core to work with an [already bootstrapped oracle pool](#Bootstrapping-An-Oracle-Pool). This section will explain how to do so for oracles using the ERG-USD connector. The steps are exactly the same for other connectors, but simply require using that connector's prepare script.

It is assumed that you are running this oracle on Linux, and have the following prerequisites:
- Access to an [Ergo Node](https://github.com/ergoplatform/ergo) v3.3.0+ with an unlocked wallet.
- A recent stable version of the [Rust Compiler](https://www.rust-lang.org/tools/install) installed.
- The Linux CLI tool `screen` and the `libssl-dev` package on Ubuntu (aka `openssl-devel` on Fedora, and potentially slightly different on other distros) installed.

1. Clone this repository via:
```sh
git clone git@github.com:ergoplatform/oracle-core.git
```
2. Enter into the connector's script folder:
```sh
cd oracle-core/scripts/erg-usd-oracle
```
3. Run the prepare script which will automatically compile the oracle core and the connector for you:
```sh
sh prepare-erg-usd-oracle.sh
```
4. Enter into the newly created `oracle-core-deployed` folder:
```sh
cd ../../hardened-erg-usd-oracle-deployed & ls
```
5. In the node you are using for this, check swagger and run the /scan/listAll command, if it returns anything other than empty [] then please deregister all scanid's one at a time with the /scan/deregister command. To be clear, the output of /scan/listAll may have registered scans that show an id number, plug each of those id numbers into the /scan/deregister command to delete them, then run /scan/listAll one more time to verify empty set [].

6. Edit your `oracle-config.yaml` as such:

node_api_key: "the password used to unlock your node"

oracle_address: "your node address that was given during bootstrap"

Everything below can also be found here for up to date info (you should check to make sure it matches): https://explorer.ergoplatform.com/en/oracle-pool-state/ergusd

oracle_pool_participant_token: "8c27dd9d8a35aac1e3167d58858c0a8b4059b277da790552e37eba22df9b9035"

oracle_pool_nft: "011d3364de07e5a26f0c4eef0852cddb387039a921b7154ef3cab22c6eda887f"

live_epoch_contract_address: "NTkuk55NdwCXkF1e2nCABxq7bHjtinX3wH13zYPZ6qYT71dCoZBe1gZkh9FAr7GeHo2EpFoibzpNQmoi89atUjKRrhZEYrTapdtXrWU4kq319oY7BEWmtmRU9cMohX69XMuxJjJP5hRM8WQLfFnffbjshhEP3ck9CKVEkFRw1JDYkqVke2JVqoMED5yxLVkScbBUiJJLWq9BSbE1JJmmreNVskmWNxWE6V7ksKPxFMoqh1SVePh3UWAaBgGQRZ7TWf4dTBF5KMVHmRXzmQqEu2Fz2yeSLy23sM3pfqa78VuvoFHnTFXYFFxn3DNttxwq3EU3Zv25SmgrWjLKiZjFcEcqGgH6DJ9FZ1DfucVtTXwyDJutY3ksUBaEStRxoUQyRu4EhDobixL3PUWRcxaRJ8JKA9b64ALErGepRHkAoVmS8DaE6VbroskyMuhkTo7LbrzhTyJbqKurEzoEfhYxus7bMpLTePgKcktgRRyB7MjVxjSpxWzZedvzbjzZaHLZLkWZESk1WtdM25My33wtVLNXiTvficEUbjA23sNd24pv1YQ72nY1aqUHa2"

epoch_preparation_contract_address: "EfS5abyDe4vKFrJ48K5HnwTqa1ksn238bWFPe84bzVvCGvK1h2B7sgWLETtQuWwzVdBaoRZ1HcyzddrxLcsoM5YEy4UnqcLqMU1MDca1kLw9xbazAM6Awo9y6UVWTkQcS97mYkhkmx2Tewg3JntMgzfLWz5mACiEJEv7potayvk6awmLWS36sJMfXWgnEfNiqTyXNiPzt466cgot3GLcEsYXxKzLXyJ9EfvXpjzC2abTMzVSf1e17BHre4zZvDoAeTqr4igV3ubv2PtJjntvF2ibrDLmwwAyANEhw1yt8C8fCidkf3MAoPE6T53hX3Eb2mp3Xofmtrn4qVgmhNonnV8ekWZWvBTxYiNP8Vu5nc6RMDBv7P1c5rRc3tnDMRh2dUcDD7USyoB9YcvioMfAZGMNfLjWqgYu9Ygw2FokGBPThyWrKQ5nkLJvief1eQJg4wZXKdXWAR7VxwNftdZjPCHcmwn6ByRHZo9kb4Emv3rjfZE"

datapoint_contract_address: "AucEQEJ3Y5Uhmu4o8dsrHy28nRTgX5sVtXvjpMTqdMQzBR3uRVcvCFbv7SeGuPhQ16AXBP7XWdMShDdhRy4cayZgxHSkdAVuTiZRvj6WCfmhXJ4LY2E46CytRAnkiYubCdEroUUX2niMLhjNmDUn4KmXWSrKngrfGwHSaD8RJUMEp5AGADaChRU6kAnh9nstkDN3"

pool_deposit_contract_address: "4L1NEtpkMq6NeZhy2pk6omYvewovcHTm7CbxKm9djsbobAHSdDe6TVfmnW5THVpSHrG6rWovqv7838reswYE3UYkykWaNnhoyBGHFCdZvWqa2TVRtHiWcVaner6giUp55ZpELLuj9TtKePv6zrtMV5YE1o2kjrQEgGDoGHBGNuyx6ymXkSimcAQo1oD4f4tTcuBfWfp"

7. Ensure your Ergo Node is running (and matches the info you input in the config) and has it's wallet unlocked (with some Ergs in the wallet to pay for tx fees).

8. Launch your oracle by running `run-oracle.sh`:
```sh
sh run-oracle.sh
```
8. A `screen` instance will be created which launches both the oracle core and the connector. (Press `Ctrl+a - n` to go between the core & the connector screens).

9. If your node is running and properly configured, the oracle core will inform you that it has successfully registered the required UTXO-set scans:
```sh
UTXO-Set Scans Have Been Successfully Registered With The Ergo Node
```

10. Press enter to confirm that the scans have been registered. Your oracle core is now properly set up and waiting for the UTXO-set scans to be triggered in order to read the state of the oracle pool on-chain to then perform actions/txs.

11. Rescan the blockchain history by either using the `/wallet/rescan` GET endpoint of your node (swagger), or by deleting `.ergo/wallet/registry` in your Ergo Node folder. Either option triggers a rescan after the blockchain progresses into the next block. There is no need to scan entire chain from height 0, simply look at your wallet in explorer and start the scan before the FIRST tx on your wallet. 

12. Once the node has finished rescanning (which can be checked via the `/wallet/status` endpoint and comparing the `walletHeight` value to the current blockheight), the oracle core & connector will automatically issue transactions and move the protocol forward (it refreshes on its own).

13. Congrats, you can now detach from the screen instance if you wish via `Ctrl+a d`. (And reattach via `screen -r`) Your oracle core/connector will run automatically going forward.

TIPS:
If you have trouble with the install and are starting over, start over from scratch (delete the entire folder) as well as deregister all the scans, OR, just deregister all the scans and delete the json file in /oracle-core-master/hardened-erg-usd-oracle. You can also make sure the numbers for node_api_key and oracle_address match the scan/listAll output after you re-scan the wallet in the steps above. You can run this as normal user. Always make sure you have no errors during the compiling in step 3 above.

# Bootstrapping An Oracle Pool
In order for an oracle pool to run, it must be first created/bootstrapped on-chain. This is the bootstrap process that is required before oracle operators can run their oracle core and have the pool function on-chain.

Check out the [Oracle Pool Bootstrap folder](oracle-pool-bootstrap) for detailed instructions about how to bootstrap an oracle pool using the CLI tool or manually.


# Writing A New Connector
If you are looking to create a new Oracle Pool for a new datapoint, you need to write a new Connector. This process has been greatly simplified thanks to [`Connector Lib`](connectors/connector-lib).

Now within 15-20 lines of Rust code, you can easily create your own Connector that plugs right in to the Oracle Core.

If you would like to integrate your pool with the Ergo Explorer we have also created [`Frontend Connector Lib`](connectors/frontend-connector-lib). This library builds off of `Connector Lib` and automatically provides + runs an API server which produces all of the data required for the frontend.

Building a Frontend Connector provides a single endpoint which summarizes the majority of relevant data about your Oracle Pool, and as such can also be useful if you intend to create your own custom website/frontend for showing off what is going on in your pool.
