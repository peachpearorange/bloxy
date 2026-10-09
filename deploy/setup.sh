#!/bin/sh
# Sets up (or updates) a Bloxy server on Debian, run as root:
#   sh setup.sh
# First run: prints a deploy key to add to GitHub, then re-run.
# Later runs pull the latest code, rebuild and restart.
set -eu

DOMAIN=${DOMAIN:-v2202610434077535942.quicksrv.de}
REPO=${REPO:-git@github.com:peachpearorange/bloxy.git}
BRANCH=${BRANCH:-ccr-cbea9050-s7p963}
PORT=${PORT:-7777}
HOME_DIR=/opt/bloxy

say() { printf '\n\033[1;32m== %s\033[0m\n' "$*"; }

say "Packages"
apt-get update -q
DEBIAN_FRONTEND=noninteractive apt-get install -y -q \
  git curl ca-certificates build-essential pkg-config \
  libwayland-dev libxkbcommon-dev libudev-dev libasound2-dev \
  ufw caddy unattended-upgrades

say "Firewall (ssh, http, https, $PORT for native clients)"
ufw allow OpenSSH
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow "$PORT"/tcp
ufw --force enable

say "Swap (builds need memory)"
if [ ! -f /swapfile ] && [ "$(awk '/MemTotal/ {print $2}' /proc/meminfo)" -lt 8000000 ]; then
  fallocate -l 4G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

say "Service user"
id bloxy >/dev/null 2>&1 || useradd --system --create-home --home-dir "$HOME_DIR" --shell /bin/sh bloxy

as_bloxy() { su - bloxy -c "$1"; }

say "Deploy key"
as_bloxy 'mkdir -p ~/.ssh && chmod 700 ~/.ssh && [ -f ~/.ssh/id_ed25519 ] || ssh-keygen -q -t ed25519 -N "" -C bloxy-server -f ~/.ssh/id_ed25519'
as_bloxy 'ssh-keyscan -t ed25519 github.com >> ~/.ssh/known_hosts 2>/dev/null; sort -u -o ~/.ssh/known_hosts ~/.ssh/known_hosts'
if ! as_bloxy "git ls-remote $REPO >/dev/null 2>&1"; then
  echo
  echo "Add this key on GitHub: repo Settings > Deploy keys > Add deploy key (leave 'Allow write access' off):"
  echo
  cat "$HOME_DIR/.ssh/id_ed25519.pub"
  echo
  echo "Then run this script again."
  exit 0
fi

say "Rust"
as_bloxy '[ -x ~/.cargo/bin/cargo ] || curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal'

say "Code ($BRANCH)"
as_bloxy "[ -d ~/bloxy ] || git clone $REPO ~/bloxy"
as_bloxy "cd ~/bloxy && git fetch origin $BRANCH && git checkout -B $BRANCH origin/$BRANCH"

say "Build (first time takes a while)"
as_bloxy 'cd ~/bloxy && ~/.cargo/bin/cargo build'

say "Service"
cat > /etc/systemd/system/bloxy.service <<UNIT
[Unit]
Description=Bloxy game server
After=network-online.target

[Service]
User=bloxy
WorkingDirectory=$HOME_DIR/bloxy
Environment=BLOXY={serve: $PORT}
Environment=RUST_LOG=info
ExecStart=$HOME_DIR/bloxy/target/debug/bloxy
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable bloxy
systemctl restart bloxy

say "Caddy (wss://$DOMAIN)"
cat > /etc/caddy/Caddyfile <<CADDY
$DOMAIN {
	reverse_proxy localhost:$PORT
}
CADDY
systemctl enable caddy
systemctl reload caddy || systemctl restart caddy

sleep 3
say "Status"
systemctl --no-pager --lines=5 status bloxy || true
echo
echo "Browser: ?connect=wss://$DOMAIN"
echo "Native:  BLOXY='{connect: \"ws://$DOMAIN:$PORT\"}' cargo run"
echo "Logs:    journalctl -u bloxy -f"
