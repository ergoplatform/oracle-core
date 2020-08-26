# This script builds the oracle core & the erg-usd connector
# and then copies the executables + config file into a final
# `erg-usd-oracle-deployed` folder.

mkdir ../../erg-usd-oracle-deployed
cp run-oracle.sh oracle-config.yaml ../../erg-usd-oracle-deployed
cd ../..
cp oracle-config.yaml erg-usd-oracle-deployed
cargo build --release
cp target/release/oracle-core erg-usd-oracle-deployed
cd connectors/erg-usd-connector
cargo build --release
cp target/release/erg-usd-connector ../../erg-usd-oracle-deployed
cd ../../erg-usd-oracle-deployed