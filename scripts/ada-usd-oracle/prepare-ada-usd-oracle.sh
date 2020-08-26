# This script builds the oracle core & the ada-usd connector
# and then copies the executables + config file into a final
# `ada-usd-oracle-deployed` folder.

mkdir ../../ada-usd-oracle-deployed
cp run-oracle.sh ../../ada-usd-oracle-deployed
cd ../..
cp oracle-config.yaml ada-usd-oracle-deployed
cargo build --release
cp target/release/oracle-core ada-usd-oracle-deployed
cd connectors/ada-usd-connector
cargo build --release
cp target/release/ada-usd-connector ../../ada-usd-oracle-deployed
cd ../../ada-usd-oracle-deployed