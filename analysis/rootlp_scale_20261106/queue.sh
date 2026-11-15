#!/bin/bash
# 300 秒の比べ: 問題ごとに base と fix を交互に (1 回目)
cd "$(dirname "$0")"
P="momentum1 satellites2-40 app1-2 irish-electricity map10 neos-1122047 map16715-04 neos-2987310-joes neos-2746589-doon neos-3656078-kumeu neos-4722843-widden neos-4763324-toguru neos-5188808-nattai neos-5195221-niemur rd-rplusc-21 s100 satellites2-60-fs snp-02-004-104 triptim1 uccase12 uccase9 momentum3 neos-799711 npmv07"
REP=${REP:-0}
for p in $P; do for c in base fix; do
  grep -q "\"problem\": \"$p\", \"cfg\": \"$c\", \"rep\": $REP," b300.jsonl 2>/dev/null && continue
  python3 run.py $c $p 300 $REP b300
done; done
echo QUEUE_DONE
