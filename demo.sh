#!/usr/bin/env bash
set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
RESET='\033[0m'

step() { echo -e "\n${BOLD}=== $* ===${RESET}"; }
ok()   { echo -e "${GREEN}✓ $*${RESET}"; }
fail() { echo -e "${RED}✗ $*${RESET}"; exit 1; }

step "Build"
cargo build --quiet
ok "build completato"

step "Test suite (unit + integrazione)"
cargo test --quiet 2>&1
ok "tutti i test passano"

step "Smoke test echo server"
RUST_LOG=info cargo run --quiet --bin echo_server &
SERVER_PID=$!
trap "kill \$SERVER_PID 2>/dev/null; wait \$SERVER_PID 2>/dev/null || true" EXIT
sleep 0.8

MSG="tokio-lab-demo"
if command -v nc &>/dev/null; then
    # Tenta con flag -w (BSD/GNU) o -q (GNU)
    RESP=$(printf '%s' "$MSG" | nc -w2 127.0.0.1 8080 2>/dev/null | head -c ${#MSG} || true)
    if [ "$RESP" = "$MSG" ]; then
        ok "echo OK: '$RESP'"
    else
        fail "echo FALLITO (risposta: '$RESP')"
    fi
elif command -v python3 &>/dev/null; then
    RESP=$(python3 -c "
import socket, sys
s = socket.create_connection(('127.0.0.1', 8080), timeout=2)
s.sendall(b'${MSG}')
data = s.recv(${#MSG})
s.close()
sys.stdout.buffer.write(data)
")
    if [ "$RESP" = "$MSG" ]; then
        ok "echo OK: '$RESP'"
    else
        fail "echo FALLITO (risposta: '$RESP')"
    fi
else
    echo "nc e python3 non trovati — smoke test manuale saltato"
    echo "Testa manualmente: echo -n '${MSG}' | nc 127.0.0.1 8080"
fi

kill -INT "$SERVER_PID"
wait "$SERVER_PID" 2>/dev/null || true
trap - EXIT

step "Demo: prodcons bounded (mpsc, backpressure)"
cargo run --quiet --bin prodcons_bounded

step "Demo: prodcons broadcast (3 consumer)"
cargo run --quiet --bin prodcons_broadcast

echo -e "\n${GREEN}${BOLD}✓ Demo completato.${RESET}"
