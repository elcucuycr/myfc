#!/bin/bash
set -x
set -e

deploySmartContract() {
  contractName=mycontract
  smartContractDir=$SCRIPTDIR/contracts/$contractName
  

  cd $smartContractDir
  RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release --lib
  cd -

  for((i=0;i<$1;i++)); do
    # per chain
    homeFlag="--home $WASMD_DATA/ibc-$i"
    rpcFlag="--node http://127.0.0.1:2655$i"
    wasmBinary="$smartContractDir/target/wasm32-unknown-unknown/release/$contractName.wasm"
    wasmd $homeFlag tx wasm store $wasmBinary $rpcFlag --from user --chain-id ibc-$i --gas-prices "0.025stake" --gas "20000000" --broadcast-mode block -y --keyring-backend test
    codeId=$(wasmd $homeFlag query wasm list-code $rpcFlag --output json | jq -r ".code_infos[-1] | .code_id")
    initMsg='{"admins": ["wasm1rcweqkrqswyaudxy5v7gsa5mygyfdhtsvhk5r2"], "donation_denom": "lala"}'
    wasmd $homeFlag tx wasm instantiate $codeId "$initMsg" $rpcFlag --from user --chain-id ibc-$i $GAS_FLAG --broadcast-mode block -y --keyring-backend test --label "hello" --no-admin
    # query the contract instance address
    contractAddr=$(wasmd $homeFlag query wasm list-contract-by-code $codeId $rpcFlag --output json | jq -r '.contracts[-1]')
    queryMsg='{ "admins_list": {} }'
    wasmd $homeFlag query wasm contract-state smart $contractAddr "$queryMsg" $rpcFlag
  done
}

# input check
chainNum=$1
if [ -z $1 ]; then
    echo "Need Number of nodes for deploying..." >&2
    exit 1
fi

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
WASMD_DATA="${SCRIPTDIR}/data"
RELAYER_HOME="${SCRIPTDIR}/rly_data"
# RELAYER_HOME="$HOME/.relayer"
GAS_FLAG='--gas-prices 0.025stake --gas 20000000 --gas-adjustment 1.1'

# preparation
if ! [ -x "$(which wasmd)" ]; then
  echo "wasmd unavailable" >&2
  exit 1
fi
if [[ ! -x "$(which jq)" ]]; then
  echo "jq (a tool for parsing json in the command line) unavailable" >&2
  exit 1
fi
if [[ ! -x "$(which jq)" ]]; then
  echo "jq (a tool for parsing json in the command line) unavailable" >&2
  exit 1
fi

# Deleted wasmd_data and relayer folders
rm -rf $WASMD_DATA
rm -rf $RELAYER_HOME
# Stop existing wasmd processes
killall wasmd || true

# start chains
mkdir $WASMD_DATA
echo "starting $chainNum chains..."
for ((i=0;i<chainNum;i++)); do
    chainId="ibc-$i"
    ./oneChain.sh wasmd $chainId $WASMD_DATA/$chainId $(expr 26550 + $i) $(expr 26660 + $i) $(expr 6060 + $i) $(expr 9090 + $i)
    # ./one-chain wasmd ibc-$i ./data 26550 26660 6060 9090
done

#  config relayer
echo "Generating rly configurations..."
mkdir $RELAYER_HOME
rly --home $RELAYER_HOME config init
for ((i=0;i<chainNum;i++)); do
    rly --home $RELAYER_HOME chains add -f configs/wasmd/chains/ibc-$i.json
    seed=$(jq -r '.mnemonic' $WASMD_DATA/ibc-$i/testkey_seed.json)
    echo "Key $(rly --home $RELAYER_HOME keys restore ibc-$i testkey "$seed") imported from ibc-$i to relayer..."
    # establish path of ibc-i with ibc-0 (coordinator)
    if [ $i -ne 0 ]; then
      rly --home $RELAYER_HOME paths new ibc-0 ibc-$i mypath0-$i
      rly --home $RELAYER_HOME transact link mypath0-$i
    fi
    # delete user to rename it to ibc-$i, --home and --keyring-backend flags are necessary for wasmd
    # wasmd --home $WASMD_DATA/ibc-$i keys delete user -y --keyring-backend="test" || true
    # cat $WASMD_DATA/ibc-$i/key_seed.json | jq .mnemonic -r | wasmd --home $WASMD_DATA/ibc-$i keys add ibc-$i --recover --keyring-backend="test"
done
echo "config rly done!"

sleep 1 # wait for rpc service to work
echo "deploying smart contract..."
# chainNum and contract directory
deploySmartContract $chainNum
echo "deploy smart contract done!"
sleep 2





