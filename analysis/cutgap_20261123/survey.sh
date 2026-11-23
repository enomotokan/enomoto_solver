#!/bin/sh
# 根の下界の 3 者比べ (当方・SCIP・HiGHS、どれも 1 スレッド・根だけ・600 秒) と、HiGHS の前処理後の問題を当方に解かせた根 (hpre)。1 本ずつ順に回す。
# 使い方: sh survey.sh 問題 ...   (リポジトリの根から)
for p in "$@"; do
  python3 analysis/cutgap_20261123/ours_root.py $p def > /dev/null
  python3 analysis/cutgap_20261123/scip_root.py $p - > /dev/null 2>&1
  WRITE_PRESOLVED=1 python3 analysis/cutgap_20261123/highs_root.py $p def > /dev/null
  OURS_MPS=analysis/cutgap_20261123/presolved/$p.mps python3 analysis/cutgap_20261123/ours_root.py $p hpre > /dev/null
  echo "$p done $(date +%T)"
done
