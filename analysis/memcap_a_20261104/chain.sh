#!/bin/sh
# memcap_a の計測の順 (2026-11-04): 長い 4 問 × 2 設定 (1200 秒) の後に 58 問 1 回
cd "$(dirname "$0")/../.."
D=analysis/memcap_a_20261104
for p in radiationm40-10-02 uccase12 CMS750_4 supportcase18; do
  for c in memcap def; do python $D/run_long.py $c $p 1200; done
done
python $D/bench58.py def,memcap 0
