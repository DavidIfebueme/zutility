#!/bin/bash
set -euo pipefail

ARCHIVE=${1:?usage: remote-deploy.sh <archive path>}
BIN=/usr/local/bin/zutility-be
PREVIOUS=/usr/local/bin/zutility-be.previous
TARGET=/opt/zutility/zutility-be/target

cd /opt/zutility/zutility-be
export HOME=/home/ubuntu
CARGO_TARGET_DIR=$TARGET /home/ubuntu/.cargo/bin/cargo build --release

/opt/zutility/deploy/apply-migrations.sh /opt/zutility/zutility-be/migrations

if [ -x "$BIN" ]; then
  cp -f "$BIN" "$PREVIOUS"
fi

install -m 755 "$TARGET/release/zutility-be" "$BIN"
systemctl restart zutility-be

ready=0
for _ in $(seq 1 12); do
  if curl -fsS --max-time 5 http://127.0.0.1:3001/ops/health/ready > /dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 5
done

if [ "$ready" = "1" ]; then
  echo "deployed $(git -C /opt/zutility rev-parse --short HEAD 2>/dev/null || date -u +%FT%TZ)"
  rm -f "$ARCHIVE"
  exit 0
fi

echo "health check failed, rolling back" >&2
if [ -x "$PREVIOUS" ]; then
  install -m 755 "$PREVIOUS" "$BIN"
  systemctl restart zutility-be
  sleep 5
fi
journalctl -u zutility-be -n 40 --no-pager >&2 || true
exit 1
