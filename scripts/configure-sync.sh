#!/usr/bin/env bash
set -euo pipefail
host=${1:-root@10.11.99.1}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
read -r -p 'Anki username/email: ' username
read -r -s -p 'Anki password: ' password
printf '\n'
printf '%s\n%s\n' "$username" "$password" | ssh "${ssh_options[@]}" "$host" \
  'umask 077; mkdir -p /home/root/.ankimarkable-pure; chmod 700 /home/root/.ankimarkable-pure; cat > /home/root/.ankimarkable-pure/ankiweb.txt; chmod 600 /home/root/.ankimarkable-pure/ankiweb.txt'
unset password
printf 'Credentials saved privately on the tablet. Open Anki and tap Sync.\n'
