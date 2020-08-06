# This script builds the oracle core & the erg-usd connector
# and then copies the executables + config file into a final
# `oracle-core-deployed` folder.

mkdir ../../oracle-core-deployed
cp run-oracle.sh ../../oracle-core-deployed
cd ../..
cp oracle-config.yaml oracle-core-deployed
cargo build --release
cp target/release/oracle-core oracle-core-deployed
cd connectors/erg-usd-connector
cargo build --release
cp target/release/erg-usd-connector ../../oracle-core-deployed
cd ../../oracle-core-deployed