#!/bin/bash
# batch3 の後に batch4 を回す
D=$(cd "$(dirname "$0")" && pwd)
while pgrep -f batch3.sh >/dev/null; do sleep 5; done
"$D/batch4.sh" > "$D/logs/batch4.out" 2>&1
