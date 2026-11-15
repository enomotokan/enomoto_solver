#!/bin/sh
# ROOTCAP: 対象 5 問を 600 秒・種 1 で、hdw・def・HiGHS の順に 1 つずつ回す (ソルバーは同時に 1 つ。res.jsonl にある組は飛ばす)
cd "$(dirname "$0")"
for cfg in hdw def highs; do
  for p in neos-873061 neos-3216931-puriri unitcal_7 net12 neos-4722843-widden; do
    grep -q "\"problem\": \"$p\", \"cfg\": \"$cfg\"" res.jsonl 2>/dev/null && continue
    python3 run1.py $p $cfg 1 600
  done
done
