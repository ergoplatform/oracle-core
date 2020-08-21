screen -S Ada-USD-Oracle -t 0 -A -d -m
screen -S Ada-USD-Oracle -X screen -t 1
screen -S Ada-USD-Oracle -p 0 -X stuff './oracle-core\n'
screen -S Ada-USD-Oracle -p 1 -X stuff './ada-usd-connector\n'
screen -R Ada-USD-Oracle -p 0