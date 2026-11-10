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

## Node support windows

- Zakura 1.8.5 halts at mainnet block 3,643,309, around 5 December 2026.
- Zebra 7.0.0 halts about 56 days after release, around 17 December 2026.
- Check the node version and halt height before every restart.
- Upgrade before the halt height. A halted node stops following the chain.

## NU7 confirmation policy

- 25 second blocks.
- Transparent orders require 9 confirmations.
- Shielded orders require 10 confirmations.

## API

- nginx terminates TLS for `api.zutility.xyz` and proxies to `127.0.0.1:3001`.
- Let's Encrypt certificate.
- Vercel uses `NEXT_PUBLIC_API_URL=https://api.zutility.xyz`.
