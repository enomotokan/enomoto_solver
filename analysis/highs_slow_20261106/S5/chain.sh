#!/bin/bash
# round1 (PID を引数で受け取る) の終わりを待ってから round2・round3 を順に回す
cd "$(dirname "$0")"
while kill -0 "$1" 2>/dev/null; do sleep 5; done
./round2.sh > round2.out 2>&1
./round3.sh > round3.out 2>&1
