#!/bin/sh
# memcap_a の追加 (2026-11-04): 58 問の後に、1200 秒で既定が落ちなかった 2 問を 1800 秒で
cd "$(dirname "$0")/../.."
D=analysis/memcap_a_20261104
while pgrep -f memcap_a_20261104/chain.sh > /dev/null; do sleep 30; done
python $D/run_long.py def CMS750_4 1800
python $D/run_long.py memcap CMS750_4 1800
python $D/run_long.py def uccase12 1800
