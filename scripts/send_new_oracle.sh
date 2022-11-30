#!/bin/sh
set -x #echo on

usage="Usage: $0 http://host:port api_key recipient_address oracle_token_id reward_token_id ballot_token_id"

if [ -z "$1" ]
then
  echo "No host supplied"
  echo ${usage}
  exit 1
fi

host=$1

if [ -z "$2" ]
then
  echo "No api_key supplied"
  echo ${usage}
  exit 1
fi

api_key=$2

if [ -z "$3" ]
then
  echo "No recipient_address supplied"
  echo ${usage}
  exit 1
fi

recipient_address=$3

if [ -z "$4" ]
then
  echo "No oracle_token_id supplied"
  echo ${usage}
  exit 1
fi

oracle_token_id=$4

if [ -z "$5" ]
then
  echo "No reward_token_id supplied"
  echo ${usage}
  exit 1
fi

reward_token_id=$5

if [ -z "$6" ]
then
  echo "No ballot_token_id supplied"
  echo ${usage}
  exit 1
fi

ballot_token_id=$6

curl -X POST "${host}/wallet/transaction/send" -H  "accept: application/json" -H  "api_key: ${api_key}" -H  "Content-Type: application/json" -d "{\"fee\":1000000,\"requests\":[{\"address\":\"${recipient_address}\",\"value\":1000000,\"assets\":[{\"tokenId\":\"${oracle_token_id}\",\"amount\":1},{\"tokenId\":\"${reward_token_id}\",\"amount\":1},{\"tokenId\":\"${ballot_token_id}\",\"amount\":1}]}]}"
