#!/bin/sh
# One-time setup of a Debian host for Bloxy, run as root:  sh setup.sh
# GitHub Actions (.github/workflows/server.yml) builds the server and uploads it
# to /opt/bloxy/bin/bloxy as the bloxy user; a path unit restarts the service.
set -eu

DOMAIN=${DOMAIN:-v2202610434077535942.quicksrv.de}
PORT=${PORT:-7777}
HOME_DIR=/opt/bloxy
KEY=/root/bloxy-deploy-key

say() { printf '\n\033[1;32m== %s\033[0m\n' "$*"; }

say "Packages"
apt-get update -q
DEBIAN_FRONTEND=noninteractive apt-get install -y -q ca-certificates openssh-client ufw caddy unattended-upgrades

say "Firewall (ssh, http, https, $PORT for native clients)"
ufw allow OpenSSH
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow "$PORT"/tcp
ufw --force enable

say "Service user"
id bloxy >/dev/null 2>&1 || useradd --system --create-home --home-dir "$HOME_DIR" --shell /bin/sh bloxy
install -d -o bloxy -g bloxy -m 755 "$HOME_DIR/bin" "$HOME_DIR/world"
install -d -o bloxy -g bloxy -m 700 "$HOME_DIR/.ssh"

say "Deploy key for GitHub Actions"
[ -f "$KEY" ] || ssh-keygen -q -t ed25519 -N "" -C bloxy-actions -f "$KEY"
grep -qF "$(cut -d' ' -f2 "$KEY.pub")" "$HOME_DIR/.ssh/authorized_keys" 2>/dev/null || cat "$KEY.pub" >> "$HOME_DIR/.ssh/authorized_keys"
chown bloxy:bloxy "$HOME_DIR/.ssh/authorized_keys"
chmod 600 "$HOME_DIR/.ssh/authorized_keys"

say "Services"
cat > /etc/systemd/system/bloxy.service <<UNIT
[Unit]
Description=Bloxy game server
After=network-online.target
ConditionPathExists=$HOME_DIR/bin/bloxy

[Service]
User=bloxy
WorkingDirectory=$HOME_DIR/world
Environment="BLOXY={serve: $PORT}"
Environment=RUST_LOG=info
ExecStart=$HOME_DIR/bin/bloxy
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
UNIT
cat > /etc/systemd/system/bloxy-restart.service <<UNIT
[Unit]
Description=Restart Bloxy after a deploy

[Service]
Type=oneshot
ExecStart=/bin/systemctl restart bloxy.service
UNIT
cat > /etc/systemd/system/bloxy-restart.path <<UNIT
[Unit]
Description=Watch for a newly deployed Bloxy server

[Path]
PathChanged=$HOME_DIR/bin/bloxy

[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable bloxy.service
systemctl enable --now bloxy-restart.path
[ -x "$HOME_DIR/bin/bloxy" ] && systemctl restart bloxy.service || true

say "Caddy (wss://$DOMAIN)"
cat > /etc/caddy/Caddyfile <<CADDY
$DOMAIN {
	reverse_proxy localhost:$PORT
}
CADDY
systemctl enable caddy
systemctl reload caddy || systemctl restart caddy

say "Done"
echo "Add this line (the deploy private key, base64) as the repository secret DEPLOY_KEY"
echo "(GitHub: repo Settings > Secrets and variables > Actions > New repository secret):"
echo
base64 -w0 "$KEY"
echo
echo
echo "It should end in: $(base64 -w0 "$KEY" | tail -c 12)"
echo
echo "Then run the 'Server' workflow (Actions tab) or push. Logs: journalctl -u bloxy -f"
