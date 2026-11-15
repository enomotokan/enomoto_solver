#!/bin/bash
# キュー 2 (2026-11-06): キュー 1 の後に、Mittelmann のうち一般整数列と 0-1 列を両方持ち、キュー 1 のふるいに入れていない 20 問を mc で 60 秒 1 回
# (screen60b.jsonl)。試作の分離が候補を出した問題だけ def でも 1 回。
cd "$(dirname "$0")"
while pgrep -f queue1.sh > /dev/null; do sleep 30; done
for p in enlight_hard graphdraw-domain neos-3046615-murg ic97_potential neos-4338804-snowy 50v-10 neos-3381206-awhea icir97_tension rococoC10-001000 wachplan rococoB10-011000 nursesched-sprint02 comp21-2idx neos-3004026-krka comp07-2idx neos-662469 supportcase33 fiball radiationm18-12-05 radiationm40-10-02; do
  python run.py mc $p 60 1 screen60b.jsonl
  if zcat logs/mc_${p}_60_r1.err.gz | grep "SEP mushagg" | awk '$NF > 0 {k=1} END {exit !k}'; then python run.py def $p 60 1 screen60b.jsonl; fi
done
