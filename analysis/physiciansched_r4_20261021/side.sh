#!/bin/bash
# side.sh: 2 巡目に時間がかかる問題 (screen2 で 0.1 秒以上) で既定と試作を比べる (60 秒、各 1 回)
cd "$(dirname "$0")"
for p in nw04 air03 mzzv42z neos-860300 hypothyroid-k1 bppc4-08; do
./run_cfg.sh $p 60 def
./run_cfg.sh $p 60 e2c ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1
done
