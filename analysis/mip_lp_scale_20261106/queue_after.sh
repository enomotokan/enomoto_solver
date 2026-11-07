#!/bin/bash
# 58 問の後: 絞った形 (6 問 × 2 回) と、ログの切れた momentum1・uccase12 の試作の取り直し (回 1)
cd "$(dirname "$0")"
python3 bench_gate.py 0
python3 bench_gate.py 1
grep -q '"problem": "momentum1", "cfg": "fix", "rep": 1' big.jsonl || python3 run1.py fix momentum1 300 1 big ENOMOTO_MIP_LPSCALE=1 ENOMOTO_MIP_LPSCALE_LOG=1
grep -q '"problem": "uccase12", "cfg": "fix", "rep": 1' big.jsonl || python3 run1.py fix uccase12 300 1 big ENOMOTO_MIP_LPSCALE=1 ENOMOTO_MIP_LPSCALE_LOG=1
