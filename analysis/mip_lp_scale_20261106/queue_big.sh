#!/bin/bash
# 6 問を 4 スレッド・300 秒で base (既定) と fix (ENOMOTO_MIP_LPSCALE=1、条件なし) で 1 回ずつ。結果は big.jsonl、ログは logs/big_*
cd "$(dirname "$0")"
for p in momentum1 satellites2-40 uccase12 opm2-z10-s4 supportcase42 rail01; do
  grep -q "\"problem\": \"$p\", \"cfg\": \"base\"" big.jsonl 2>/dev/null || python3 run1.py base $p 300 0 big
  grep -q "\"problem\": \"$p\", \"cfg\": \"fix\"" big.jsonl 2>/dev/null || python3 run1.py fix $p 300 0 big ENOMOTO_MIP_LPSCALE=1 ENOMOTO_MIP_LPSCALE_LOG=1
done
# 再起動で既定の satellites2-40 のログが切れたので取り直す (回 1)
grep -q "\"problem\": \"satellites2-40\", \"cfg\": \"base\", \"rep\": 1" big.jsonl 2>/dev/null || python3 run1.py base satellites2-40 300 1 big
