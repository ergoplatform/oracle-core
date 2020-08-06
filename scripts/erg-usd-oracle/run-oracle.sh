screen -S OracleCore -t 0 -A -d -m
screen -S OracleCore -X screen -t 1
screen -S OracleCore -p 0 -X stuff './oracle-core\n'
screen -S OracleCore -p 1 -X stuff './erg-usd-connector\n'
screen -R OracleCore -p 0