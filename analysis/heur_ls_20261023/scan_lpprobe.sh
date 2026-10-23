#!/bin/bash
# 前処理の LP による固定の最初の LP の時間を全問で調べる (既定、60 秒の設定で前処理の後まで 1 問 25 秒で打ち切る)
# 出力: 問題 / 前処理の LP の行
cd "$(dirname "$0")"
for p in $(ls ../../.miplib_cache/*.mps | xargs -n1 basename | sed 's/.mps$//'); do
  timeout 25 env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python3 eno_run.py ../../.miplib_cache/$p.mps 60 4 > logs/scan_${p}.log 2>&1
  echo "$p | $(grep -m1 -E 'LP probing: (root LP|the first LP)' logs/scan_${p}.log) | $(grep -m1 '^Presolve:' logs/scan_${p}.log)"
done
