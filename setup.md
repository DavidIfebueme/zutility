# Zutility — Full Linode Setup Guide

Single 8GB Linode running zcashd + backend + PostgreSQL + Nginx. Frontend on Vercel.

Recommended Linode: 8GB RAM, 4 CPU, 500GB SSD (Dedicated CPU 8GB or Shared 8GB).

---

## Step 1 — Provision the Linode

1. Go to https://cloud.linode.com/linodes/create
2. Choose **Dedicated CPU 8GB** (or Shared 8GB if budget is tight)
3. Image: **Ubuntu 24.04 LTS**
4. Region: closest to Nigeria (eu-west = London, or ap-south = Mumbai)
5. Set root password (save it)
6. Add your SSH key
7. Create

After provisioning, note the **Public IP** (e.g. `192.0.2.50`). SSH in:

```bash
ssh root@YOUR_LINODE_IP
```

Update the system:

```bash
apt update && apt upgrade -y
apt install -y build-essential pkg-config libc6-dev m4 g++-multilib \
  autoconf libtool ncurses-dev unzip git python3 zlib1g-dev wget \
  bsdmainutils automake libboost-all-dev libssl-dev libprotobuf-dev \
  protobuf-compiler libevent-dev cmake curl libpq-dev postgresql-client \
  docker.io docker-compose-plugin nginx certbot python3-certbot-nginx \
  ufw fail2ban
```

Set up firewall (Nginx must be installed first for the profile to exist):

```bash
ufw allow OpenSSH
ufw allow 'Nginx Full'
ufw allow 18232/tcp
ufw --force enable
```

If `ufw allow 'Nginx Full'` fails with "Could not find a profile", run:

```bash
apt install -y nginx
ufw app list
ufw allow 'Nginx Full'
```

---

## Step 2 — Build zcashd from Source on the Linode

This takes 2-4 hours on a 4-core Linode. Start it first so it can sync while you set up the rest.

```bash
cd /opt
git clone https://github.com/zcash/zcash.git
cd zcash
git checkout v6.1.0
```

Build (uses all cores, takes 1-2 hours):

```bash
apt install -y make
./zcutil/build.sh -j$(nproc)
```

Create config directory and config file:

```bash
mkdir -p ~/.zcash
cat > ~/.zcash/zcash.conf << 'EOF'
testnet=1
server=1
rpcuser=zutility_rpc
rpcpassword=CHANGE_THIS_TO_A_LONG_RANDOM_STRING_64_CHARS
rpcallowip=127.0.0.1
rpcbind=127.0.0.1
rpcport=18232
txindex=1
addnode=testnet.zcashnode.com:18232
addnode=testnet.seeder.zcashnode.com:18232
EOF
```

Generate a real RPC password:

```bash
openssl rand -hex 32
```

Replace `CHANGE_THIS_TO_A_LONG_RANDOM_STRING_64_CHARS` with the output. Save this password — it goes in `ZCASH_RPC_PASSWORD` in your `.env`.

Start zcashd in a tmux session so it keeps running:

```bash
apt install -y tmux
tmux new -s zcashd
/opt/zcash/src/zcashd -daemon -conf=/root/.zcash/zcash.conf -datadir=/root/.zcash
```

Detach from tmux with `Ctrl+B` then `D`. Reattach later with `tmux attach -t zcashd`.

Check sync progress:

```bash
/opt/zcash/src/zcash-cli -testnet getblockchaininfo
```

Look for `"initialblockdownload": true` — when it becomes `false`, sync is done. Testnet sync takes 2-6 hours.

Make zcashd auto-start on reboot:

```bash
cat > /etc/systemd/system/zcashd.service << 'EOF'
[Unit]
Description=zcashd testnet
After=network.target

[Service]
Type=forking
User=root
ExecStart=/opt/zcash/src/zcashd -daemon -conf=/root/.zcash/zcash.conf -datadir=/root/.zcash
ExecStop=/opt/zcash/src/zcash-cli -testnet stop
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable zcashd
```

---

## Step 3 — Set Up PostgreSQL

Run PostgreSQL in Docker:

```bash
mkdir -p /opt/postgres-data

docker run -d \
  --name zutility-postgres \
  --restart unless-stopped \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=zutility \
  -p 127.0.0.1:5432:5432 \
  -v /opt/postgres-data:/var/lib/postgresql/data \
  postgres:16-alpine
```

Verify:

```bash
docker ps
PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d zutility -c "SELECT 1;"
```

---

## Step 4 — Install Rust and Build the Backend

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

Clone your repo (or push from local and pull):

```bash
cd /opt
git clone https://github.com/YOUR_USERNAME/zutility.git
cd /opt/zutility/zutility-be
```

Install SQLx CLI for migrations:

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

Create the `.env` file (see Step 5 below for how to get each value):

```bash
cat > /opt/zutility/zutility-be/.env << 'ENVEOF'
APP_ENV=dev
HTTP_BIND_ADDR=0.0.0.0:3001
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/zutility
ORDER_TOKEN_HMAC_SECRET=
IP_HASH_SECRET=

VTPASS_BASE_URL=https://sandbox.vtpass.com/api
VTPASS_API_KEY=
VTPASS_SECRET_KEY=

REMITA_MERCHANT_ID=
REMITA_API_KEY=
REMITA_SERVICE_TYPE_ID=
REMITA_BASE_URL=https://remitademo.net/remita/exapp/api/v1
REMITA_WEBHOOK_SECRET=

ZCASH_RPC_MODE=tcp
ZCASH_RPC_SOCKET_PATH=
ZCASH_RPC_URL=http://127.0.0.1:18232
ZCASH_RPC_USER=zutility_rpc
ZCASH_RPC_PASSWORD=
ZCASH_NETWORK=testnet
REQUIRED_CONFS_TRANSPARENT=3
REQUIRED_CONFS_SHIELDED=10
ORDER_EXPIRY_MINUTES=30
RATE_LOCK_MINUTES=15
SWEEP_THRESHOLD_ZEC=0.5
SIGNING_SERVICE_URL=http://127.0.0.1:8080
SIGNING_SERVICE_HMAC_SECRET=
RATE_SOURCE_TIMEOUT_MS=3000
ENVEOF
```

Build release binary (takes 10-20 min):

```bash
cd /opt/zutility/zutility-be
cargo build --release
```

Run migrations:

```bash
cd /opt/zutility/zutility-be
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/zutility sqlx migrate run
```

Seed utilities:

```bash
PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d zutility -f /opt/zutility/zutility-be/seeds/seed_utilities.sql
```

---

## Step 5 — Get All Environment Variable Values

### Auto-generated secrets

Generate these by running `openssl rand -hex 32` once per secret:

| Variable | How to get |
|---|---|
| `ORDER_TOKEN_HMAC_SECRET` | `openssl rand -hex 32` — paste the output |
| `IP_HASH_SECRET` | `openssl rand -hex 32` — paste the output |
| `REMITA_WEBHOOK_SECRET` | `openssl rand -hex 32` — paste the output |
| `SIGNING_SERVICE_HMAC_SECRET` | `openssl rand -hex 32` — paste the output |

Run this to generate all four at once:

```bash
for i in ORDER_TOKEN_HMAC_SECRET IP_HASH_SECRET REMITA_WEBHOOK_SECRET SIGNING_SERVICE_HMAC_SECRET; do
  echo "$i=$(openssl rand -hex 32)"
done
```

Copy each line into your `.env` file.

### VTpass keys

| Variable | How to get |
|---|---|
| `VTPASS_API_KEY` | See below |
| `VTPASS_SECRET_KEY` | See below |

Steps:
1. Open https://sandbox.vtpass.com/register in your browser
2. Fill in the form — use any email/password (this is sandbox, no verification needed)
3. After registration, log in at https://sandbox.vtpass.com/login
4. Click your profile name (top right) → **API Keys**
5. You will see **Public Key (API Key)** and **Secret Key**
6. Copy **Public Key** → paste as `VTPASS_API_KEY` in `.env`
7. Copy **Secret Key** → paste as `VTPASS_SECRET_KEY` in `.env`
8. The sandbox gives you free test balance automatically — no funding needed

For production later:
1. Go to https://vtpass.com/register instead
2. After signup, go through their merchant verification (business name, CAC, etc.)
3. Same path: Profile → API Keys
4. Change `VTPASS_BASE_URL` to `https://vtpass.com/api`
5. Fund your wallet: Dashboard → Wallet → Fund Wallet → bank transfer or card

### Remita keys

| Variable | How to get |
|---|---|
| `REMITA_MERCHANT_ID` | See below |
| `REMITA_API_KEY` | See below |
| `REMITA_SERVICE_TYPE_ID` | See below |

Steps:
1. Open https://www.remita.net/merchant/signup in your browser
2. Fill in business details — for sandbox/testing, you can use test info
3. After signup, log into the Remita merchant portal
4. Navigate to **Profile** or **Settings** → **API Credentials**
5. You will see **Merchant ID** → copy as `REMITA_MERCHANT_ID`
6. You will see **API Key** → copy as `REMITA_API_KEY`
7. For **Service Type ID**: go to **Billers** → **Service Types**. If you registered as a school fees biller, you'll see your service type ID there. For initial testing, use the generic test service type ID that Remita provides in their sandbox documentation. Contact Remita support at support@remita.net if you don't see one.
8. The demo/sandbox base URL is already set: `https://remitademo.net/remita/exapp/api/v1`

Note: Remita sandbox access can take 1-2 business days to get approved. You can start testing with VTpass only while waiting.

For production later:
1. Remita will verify your business (CAC, bank account, etc.)
2. They issue live credentials
3. Change `REMITA_BASE_URL` to the production URL Remita gives you
4. Each school on Remita has its own `service_type_id` — you'll need to register individual schools or use their generic bill payment endpoint

### Zcash RPC credentials

| Variable | How to get |
|---|---|
| `ZCASH_RPC_USER` | Set by you in `~/.zcash/zcash.conf` — we used `zutility_rpc` |
| `ZCASH_RPC_PASSWORD` | The password you generated with `openssl rand -hex 32` and put in `~/.zcash/zcash.conf` |

These must match what's in `/root/.zcash/zcash.conf`. If you followed Step 2, `ZCASH_RPC_USER=zutility_rpc` and `ZCASH_RPC_PASSWORD` is the random string you generated.

### Frontend environment variables (Vercel)

| Variable | Value |
|---|---|
| `NEXT_PUBLIC_API_URL` | `https://api.zutility.xyz` (after you set up DNS in Step 7) |
| `NEXT_PUBLIC_WS_URL` | `wss://api.zutility.xyz` |

Set these in: Vercel Dashboard → your project → Settings → Environment Variables

---

## Step 6 — Fill In the .env File

Open the `.env` and fill in all the values you gathered:

```bash
nano /opt/zutility/zutility-be/.env
```

Replace every empty value after `=` with the real key/secret from Step 5.

Verify it looks right (no empty values except optional ones):

```bash
grep -c '=$' /opt/zutility/zutility-be/.env
```

Should output `0` (no empty values).

---

## Step 7 — Create the Backend systemd Service

```bash
cat > /etc/systemd/system/zutility-be.service << 'EOF'
[Unit]
Description=Zutility Backend
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/zutility/zutility-be
EnvironmentFile=/opt/zutility/zutility-be/.env
ExecStart=/opt/zutility/zutility-be/target/release/zutility-be
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable zutility-be
systemctl start zutility-be
```

Check it's running:

```bash
systemctl status zutility-be
journalctl -u zutility-be -f
```

Test the API:

```bash
curl http://127.0.0.1:3001/api/v1/utilities
```

You should get a JSON array of utilities.

---

## Step 8 — Set Up DNS and Nginx

Point your domain to the Linode:

1. Go to your domain registrar (where you bought zutility.xyz)
2. Create an **A Record**: `api.zutility.xyz` → your Linode IP
3. Wait for DNS propagation (usually 5-30 min, can take up to 48 hours)

Verify DNS:

```bash
dig api.zutility.xyz
```

Should resolve to your Linode IP.

Create Nginx config:

```bash
cat > /etc/nginx/sites-available/zutility << 'EOF'
server {
    listen 80;
    server_name api.zutility.xyz;

    location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }
}
EOF

ln -sf /etc/nginx/sites-available/zutility /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default
nginx -t
systemctl reload nginx
```

Get HTTPS certificate:

```bash
certbot --nginx -d api.zutility.xyz
```

Follow the prompts — select "Redirect HTTP to HTTPS".

Verify:

```bash
curl https://api.zutility.xyz/api/v1/utilities
```

Should return the same JSON array as before, now over HTTPS.

---

## Step 9 — Update Vercel Environment Variables

1. Go to https://vercel.com/dashboard
2. Click your zutility-fe project
3. Settings → Environment Variables
4. Set:
   - `NEXT_PUBLIC_API_URL` = `https://api.zutility.xyz`
   - `NEXT_PUBLIC_WS_URL` = `wss://api.zutility.xyz`
5. Go to Deployments → click the three dots on the latest deployment → Redeploy

---

## Step 10 — Get Testnet ZEC

You need testnet ZEC to test end-to-end.

Option A — Faucet:
1. Search "Zcash testnet faucet" in your browser
2. Common faucet: https://faucet.zecpages.com (if available)
3. Enter your testnet t-address (generate one on the Linode):
   ```bash
   /opt/zcash/src/zcash-cli -testnet getnewaddress
   ```
4. The faucet sends testnet ZEC to that address

Option B — Zashi wallet:
1. Download Zashi on your phone
2. Switch to testnet mode in settings
3. Use the built-in faucet in Zashi (if available)
4. Or receive from another testnet wallet

Check your balance on the Linode:

```bash
/opt/zcash/src/zcash-cli -testnet getbalance
```

---

## Step 11 — End-to-End Test

1. Visit https://zutility.xyz/pay
2. Select **MTN Airtime**
3. Enter a phone number (any Nigerian format like `08012345678`)
4. Enter an amount (e.g., ₦100)
5. Choose **Transparent** address type (faster confirmations on testnet)
6. Click **Create Order**
7. You'll see a deposit address and ZEC amount
8. Open your testnet wallet (Zashi on testnet, or use zcash-cli)
9. Send the exact ZEC amount to the deposit address:
   ```bash
   /opt/zcash/src/zcash-cli -testnet sendtoaddress "DEPOSIT_ADDRESS" AMOUNT
   ```
10. Watch the order progress in the UI:
    - `awaiting_payment` → `payment_detected` → `payment_confirmed` → `utility_dispatching` → `completed`
11. On completion, you'll see the delivery reference (airtime reference, electricity token, WAEC PIN, etc.)

For electricity specifically:
1. Select an electricity DISCO (e.g., Ikeja Electric)
2. Enter a meter number (use a test meter from VTpass sandbox docs)
3. Click **Validate** to verify the meter
4. Enter amount and create order
5. On completion, you'll see the prepaid meter token

---

## Step 12 — Monitor and Maintain

### Check all services are running

```bash
systemctl status zcashd
systemctl status zutility-be
docker ps
```

### View backend logs

```bash
journalctl -u zutility-be -f
```

### View zcashd logs

```bash
tmux attach -t zcashd
# or
tail -f /root/.zcash/testnet3/debug.log
```

### Restart backend after code changes

```bash
cd /opt/zutility/zutility-be
git pull
cargo build --release
systemctl restart zutility-be
```

### Restart zcashd

```bash
systemctl restart zcashd
```

### Database backup

```bash
docker exec zutility-postgres pg_dump -U postgres zutility > /opt/backups/zutility_$(date +%Y%m%d).sql
```

Set up a cron job for daily backups:

```bash
mkdir -p /opt/backups
(crontab -l 2>/dev/null; echo "0 3 * * * docker exec zutility-postgres pg_dump -U postgres zutility > /opt/backups/zutility_\$(date +\%Y\%m\%d).sql") | crontab -
```

---

## VTpass Service IDs Reference

| Service | serviceID (slug) |
|---|---|
| MTN Airtime | mtn |
| Airtel Airtime | airtel |
| Glo Airtime | glo |
| 9mobile Airtime | 9mobile |
| MTN Data | mtn-data |
| Airtel Data | airtel-data |
| Glo Data | glo-data |
| 9mobile Data | 9mobile-data |
| DSTV | dstv |
| GOtv | gotv |
| Startimes | startimes |
| Showmax | showmax |
| WAEC Registration | waec-registration |
| WAEC Result Checker | waec-result-checker |
| JAMB | jamb |
| Ikeja Electric | ikeja-electric |
| Eko Electric | eko-electric |
| Abuja Electric | abuja-electric |
| Ibadan Electric | ibadan-electric |
| Kano Electric | kano-electric |
| PH Electric | phed-electric |
| Jos Electric | jos-electric |
| Kaduna Electric | kaduna-electric |
| Enugu Electric | enugu-electric |
| Benin Electric | benin-electric |
| Yola Electric | yola-electric |
| Aba Electric | aba-electric |

---

## Provider Routing Reference

| Utility Type | Primary Provider | Fallback |
|---|---|---|
| airtime | VTpass | — |
| data | VTpass | — |
| dstv, gotv, startimes, showmax | VTpass | — |
| electricity | VTpass | Remita |
| waec, jamb | VTpass | — |
| school_fees | Remita | — |

---

## Moving to Mainnet

When ready to switch from testnet to mainnet:

1. Change `ZCASH_NETWORK=testnet` to `ZCASH_NETWORK=mainnet` in `.env`
2. Update `ZCASH_RPC_URL` port if needed (mainnet default is 8232)
3. Update `~/.zcash/zcash.conf` — remove `testnet=1`, change `rpcport=8232`, update `addnode` to mainnet peers
4. Restart zcashd — mainnet sync takes 1-3 days
5. Increase `REQUIRED_CONFS_SHIELDED` to 10+ and `REQUIRED_CONFS_TRANSPARENT` to 6+ for safety
6. Switch VTpass: `VTPASS_BASE_URL=https://vtpass.com/api`
7. Switch Remita to production URL (they provide this during merchant verification)
8. Fund your VTpass wallet with real Naira (Dashboard → Wallet → Fund Wallet)
9. Fund your Remita settlement account with real Naira
10. Set `APP_ENV=prod`
11. Restrict CORS in `zutility-be/src/http/mod.rs` — change `Any` to `https://zutility.xyz`
12. Set up the signing service on a separate hardened VPS with the Zcash spend key
13. Run a 7-day soak test with small real amounts before going public
14. Consider upgrading to a larger Linode (16GB RAM) — mainnet zcashd uses more memory

---

## Troubleshooting

### zcashd won't sync
- Check internet: `curl -I https://example.com`
- Check peers: `/opt/zcash/src/zcash-cli -testnet getpeerinfo`
- Add more nodes to `~/.zcash/zcash.conf`:
  ```
  addnode=testnet.zcashnode.com:18232
  addnode=testnet2.zcashnode.com:18232
  ```

### Backend won't start
- Check logs: `journalctl -u zutility-be -n 50`
- Verify `.env` has no empty values: `grep '=$' /opt/zutility/zutility-be/.env`
- Verify PostgreSQL is running: `docker ps | grep postgres`
- Verify zcashd is running: `systemctl status zcashd`

### Order stuck at "awaiting_payment"
- Check zcashd is synced: `/opt/zcash/src/zcash-cli -testnet getblockchaininfo`
- Check the deposit address received funds: `/opt/zcash/src/zcash-cli -testnet getreceivedbyaddress "ADDRESS"`
- Check backend logs for scan errors: `journalctl -u zutility-be -f`

### VTpass returns errors
- Verify API keys in `.env` match your sandbox dashboard
- Test manually: `curl -H "Authorization: Basic BASE64_OF_API_KEY:SECRET_KEY" https://sandbox.vtpass.com/api/me/services`
- Replace BASE64 with: `echo -n "YOUR_API_KEY:YOUR_SECRET_KEY" | base64`

### Remita returns auth errors
- Verify merchant ID, API key, and service type ID are correct
- Check hash computation matches their spec
- Contact Remita support: support@remita.net
