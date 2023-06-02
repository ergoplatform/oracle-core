# This script builds the oracle core & the erg-usd connector
# and then copies the executables + config file into a final
# `erg-usd-oracle-deployed` folder.

mkdir ../../hardened-erg-usd-oracle-deployed
cp run-oracle.sh oracle-config.yaml ../../hardened-erg-usd-oracle-deployed
cd ../..
#the below command is not needed as the config doesn't exist there
#cp oracle-config.yaml hardened-erg-usd-oracle-deployed
cargo build --release
cp target/release/oracle-core hardened-erg-usd-oracle-deployed
cd connectors/erg-usd-connector
cargo build --release
cp target/release/erg-usd-connector ../../hardened-erg-usd-oracle-deployed
cd ../../hardened-erg-usd-oracle-deployed
