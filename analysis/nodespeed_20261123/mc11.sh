#!/bin/bash
# 9.5 節: mc11 で既定と ENOMOTO_MIP_NODEROWS_ADAPT=r を 1・4 スレッド × 種 1〜3、120 秒で回す (計 12 回)。リポジトリの根から:
#   bash analysis/nodespeed_20261123/mc11.sh
# 結果は mc11.jsonl、ログは logs/mc11_<名札>_t<スレッド>_s<種>.log.gz (コミットしない)。NRA_TRACE で 200 ノードごとの φ・f・ρ を出す
cd "$(dirname "$0")/../.."
D=analysis/nodespeed_20261123
for th in 1 4; do for s in 1 2 3; do for v in "def:X=1" "adr:ENOMOTO_MIP_NODEROWS_ADAPT=r"; do
  tag=${v%%:*}; kv=${v#*:}
  grep -q "\"tag\": \"$tag\", \"threads\": $th, \"seed\": $s," $D/mc11.jsonl 2>/dev/null && continue
  f=$D/logs/mc11_${tag}_t${th}_s$s.log
  env ENOMOTO_MIP_NRA_TRACE=200 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s $kv python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/mc11.mps --time-limit 120 --threads $th > $f 2>&1
  r=$(grep '^{' $f | tail -1); gzip -f $f
  echo "{\"tag\": \"$tag\", \"threads\": $th, \"seed\": $s, \"result\": ${r:-null}}" >> $D/mc11.jsonl
  echo "$tag t$th s$s $(echo $r | cut -c1-120)"
done; done; done
