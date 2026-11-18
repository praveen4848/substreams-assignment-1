#!/usr/bin/env bash
set -euo pipefail

[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"

cargo build --target wasm32-unknown-unknown --release
substreams pack substreams.yaml -o solana-transfers-v0.1.0.spkg

ENDPOINT="${SUBSTREAMS_ENDPOINT:-solana.substreams.pinax.network:443}"
substreams run solana-transfers-v0.1.0.spkg map_transfers \
  -e "$ENDPOINT" \
  -s 385870151 -t 385870157 \
  -o jsonl > raw_output.jsonl

python3 - <<'PY' > transfers.jsonl
import json
with open("raw_output.jsonl", "r", encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line: continue
        obj = json.loads(line)
        data = obj.get("@data") or obj.get("data") or {}
        for t in data.get("transfers", []):
            print(json.dumps({
                "from": t.get("from", ""),
                "to": t.get("to", ""),
                "amount": int(t.get("amount", 0)),
            }))
PY
