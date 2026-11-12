#!/bin/bash
# 手元 1 本で shards_L.json の 14 分担を順に回す。
cd "$(dirname "$0")"
for k in $(seq 0 13); do python3 run.py L $k; done
