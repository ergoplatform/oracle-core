screen -S Erg-USD-Oracle -t 0 -A -d -m
screen -S Erg-USD-Oracle -X screen -t 1
screen -S Erg-USD-Oracle -p 0 -X stuff './oracle-core\n'
screen -S Erg-USD-Oracle -p 1 -X stuff './erg-usd-connector\n'
screen -R Erg-USD-Oracle -p 0