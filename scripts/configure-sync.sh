#!/usr/bin/env bash
set -euo pipefail
host=${1:-root@10.11.99.1}
ssh_options=(-o ConnectTimeout=10)
if [[ -n ${SSH_CONTROL_PATH:-} ]]; then ssh_options+=(-S "$SSH_CONTROL_PATH"); fi
read -r -p 'Anki username/email: ' username
read -r -s -p 'Anki password: ' password
printf '\n'
printf '%s\n%s\n' "$username" "$password" | ssh "${ssh_options[@]}" "$host" \
  '/home/root/xovi/exthome/appload/ankimarkable-pure/ankimarkable-pure-tools configure-sync /home/root/.ankimarkable-pure'
unset password
printf 'Run ./ankipure sync to exchange cards and reviews with AnkiWeb.\n'
