#!/bin/bash
cd "$(dirname "$0")"
while pgrep -f "\./base\.sh" >/dev/null; do sleep 10; done
./queue.sh > queue.out 2>&1
