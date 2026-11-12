#!/bin/bash
set -euo pipefail

RPC="http://127.0.0.1:18232"
NAMESPACE="zutility/mainnet"

TOKEN=$(curl -s -X PUT "http://169.254.169.254/latest/api/token" \
  -H "X-aws-ec2-metadata-token-ttl-seconds: 60")
INSTANCE_ID=$(curl -s -H "X-aws-ec2-metadata-token: $TOKEN" \
  http://169.254.169.254/latest/meta-data/instance-id)

rpc() {
  curl -s --max-time 10 -X POST -H 'content-type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$1\",\"params\":$2}" "$RPC" || true
}

HEIGHT=$(rpc getblockchaininfo '[]' | jq -r '.result.blocks // empty')
PEERS=$(rpc getnetworkinfo '[]' | jq -r '.result.connections // empty')
DISK=$(df --output=pcent / | tail -1 | tr -dc '0-9')

METRICS=""
add() {
  local name=$1 value=$2 unit=$3
  [ -z "$value" ] && return 0
  [ -n "$METRICS" ] && METRICS="$METRICS,"
  METRICS="$METRICS{\"MetricName\":\"$name\",\"Value\":$value,\"Unit\":\"$unit\",\"Dimensions\":[{\"Name\":\"InstanceId\",\"Value\":\"$INSTANCE_ID\"}]}"
}

if [ -n "$HEIGHT" ]; then
  BEST=$(rpc getbestblockhash '[]' | jq -r '.result // empty')
  BLOCK_TIME=""
  if [ -n "$BEST" ]; then
    BLOCK_TIME=$(rpc getblock "[\"$BEST\",1]" | jq -r '.result.time // empty')
  fi
  add NodeUp 1 Count
  add BlockHeight "$HEIGHT" Count
  if [ -n "$BLOCK_TIME" ]; then
    add TipAgeSeconds "$(( $(date +%s) - BLOCK_TIME ))" Seconds
  fi
else
  add NodeUp 0 Count
fi

add PeerCount "$PEERS" Count
add DiskUsedPercent "$DISK" Percent

LWD=$(grpcurl -plaintext -max-time 10 -d '{}' 127.0.0.1:9067 \
  cash.z.wallet.sdk.rpc.CompactTxStreamer/GetLightdInfo 2>/dev/null \
  | jq -r '.latestBlockHeight // empty' || true)
add LwdHeight "$LWD" Count

[ -z "$METRICS" ] && exit 0

aws cloudwatch put-metric-data \
  --namespace "$NAMESPACE" \
  --metric-data "[$METRICS]"
