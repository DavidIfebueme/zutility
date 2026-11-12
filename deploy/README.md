# zutility deployment

## Testnet box

- AWS account `381492245784`, region `us-east-1`.
- Instance `i-08ae36b488bdfb102`, `m7i.xlarge`, 4 vCPU, 16 GB, 200 GB gp3.
- Elastic IP `100.51.169.55`. DNS `api.zutility.xyz`.
- Ubuntu 24.04.
- Zakura 1.8.5 testnet. P2P `18233`, RPC `127.0.0.1:18232`.
- lightwalletd 0.6.1. gRPC `127.0.0.1:19067`.
- Postgres 16. Database `zutility`.
- SSH key `~/.ssh/zutility-testnet.pem`. No human IAM user.
- Instance role `zutility-testnet-role`. SSM Session Manager and read access to `/zutility/*` parameters.

## Mainnet node box

Node and indexer only. No app, no Postgres, no TLS vhost, no wallet, no API keys.

- Instance `i-08b5d088aba715f04`, `m7i.xlarge`, `us-east-1a`, Ubuntu 24.04.
- Private IP `172.31.9.210`, private DNS `ip-172-31-9-210.ec2.internal`. No Elastic IP.
- Security group `zutility-mainnet` (`sg-0650bd154ce6ff9ca`): `18233/tcp` from anywhere for P2P, `9067/tcp` from the app box security group only. No SSH port.
- Access is SSM Session Manager only. `ssh.socket` is masked, so nothing listens on 22.
- 700 GB gp3, encrypted, delete on terminate. 500 MB/s throughput during the bootstrap, reverted to 125 MB/s after.
- IMDSv2 required, hop limit 1.
- Instance role `zutility-mainnet-role`: `AmazonSSMManagedInstanceCore` and `cloudwatch:PutMetricData` only.
- zakurad 1.8.6, archive mode, config `/etc/zakura/zakura.toml`, RPC `127.0.0.1:18232`, P2P `[::]:18233`, state `/var/lib/zakura`.
- lightwalletd 0.6.1 container pinned by digest `sha256:7be272c4eff6b0a8ac83b61cc98b8ffbf14f3bdad512fa6f17a8ed93d768552f`, gRPC `0.0.0.0:9067`, data `/var/lib/lightwalletd`.
- Metrics every 5 minutes to CloudWatch namespace `zutility/mainnet`: `BlockHeight`, `TipAgeSeconds`, `PeerCount`, `DiskUsedPercent`, `NodeUp`, `LwdCachedHeight`, `LwdLagBlocks`.
- Alarms to `zutility-ops-alerts`: instance status, node down, tip older than 15 minutes, disk above 85 percent, and block height at 3,600,000 as the halt warning.
- `LwdCachedHeight` comes from the indexer log because no RPC exposes the cache height. Watch `LwdLagBlocks` during a first sync.
- Cost: compute `$147.17` plus 700 GB gp3 `$56.00` is `$203.17` per month. Inbound transfer is free.

### Snapshot bootstrap, 12 November 2026

Source `https://zakura.valargroup.dev/mainnet/zakura-mainnet-20261112T140818Z-3565614.tar.zst`, height 3,565,614, 256.4 GiB, sha256 `610ccda8eff4b0fa671eeda9575c110ae7ac530b6761781efe1a4b19f57b7d47`.

```text
launch                     20:16:50 UTC
download start             20:18:22 UTC
transfer complete          about 16 minutes at 240 MB/s with aria2 -x16
sha256 verify complete     20:57:59 UTC   (22 minutes reading 275 GB)
extract complete           21:18:32 UTC   (20m33s, 264 GB of state)
node at tip                21:27 UTC      chain "main", 9 peers
bootstrap total            60 minutes download, verify, and extract
```

Raise gp3 throughput to 500 MB/s before the bootstrap and drop it back to 125 after. At the default 125 the download is disk bound.

### Teardown

```text
aws ec2 terminate-instances --instance-ids i-08b5d088aba715f04
```

The 700 GB volume has delete-on-terminate, so it goes with the instance. Nothing else points at this box, so no DNS or DNS cleanup is needed.

## Node support windows

- Zakura 1.8.6 halts at mainnet block 3,643,309, around 5 December 2026. Supported until then, with warnings from about three days earlier.
- Zebra 7.0.0 halts about 56 days after release, around 17 December 2026.
- Check the node version and halt height before every restart.
- Upgrade before the halt height. A halted node stops following the chain. An upgrade is a restart, not a resync.

## NU7 confirmation policy

- 25 second blocks.
- Mainnet: 3 transparent and 3 shielded, about 75 seconds.
- Testnet currently runs 2 transparent and 3 shielded.
- `REQUIRED_CONFS_TRANSPARENT` and `REQUIRED_CONFS_SHIELDED` set it per environment.

## Switching the app between networks

The binary is the same for both. Only environment changes.

- `ZCASH_NETWORK=mainnet|testnet` selects the chain and the wallet subdirectory.
- `ZINGO_INDEXER_URI` points at lightwalletd. On the mainnet box that is `http://172.31.9.210:9067`.
- `ZINGO_WALLET_DIR` holds both wallets, namespaced by chain, so neither is touched when flipping.
- The deposit address pool is tagged by network and refills on first boot for the new chain. Orders fail with "no unused deposit address" until it does.
- One chain is watched at a time. Orders on the other chain stop progressing while switched away.
- Startup now refuses to run when the indexer chain name does not match `ZCASH_NETWORK`, and a production start fails if the wallet cannot initialize.

## API

- nginx terminates TLS for `api.zutility.xyz` and proxies to `127.0.0.1:3001`.
- Let's Encrypt certificate.
- Vercel uses `NEXT_PUBLIC_API_URL=https://api.zutility.xyz`.
