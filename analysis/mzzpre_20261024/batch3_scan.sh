#!/bin/bash
# 副作用の洗い出し: 40 問 + 16 問 (heur_ls_20261023/bench_subsets.txt) を 20 秒ずつ、既定 (s_def) と ROWS_FIRST (s_rf) で 1 回ずつ。
# クリーク行の拡張の行と根の LP を scan.tsv に並べる
cd $(dirname $0)
for p in $(sed -n '2,$p' ../heur_ls_20261023/bench_subsets.txt | cut -d' ' -f2- | tr ' ' '\n' | sort -u); do
  [ -f ../../.miplib_cache/$p.mps ] || { echo "$p missing"; continue; }
  ./run_cfg.sh $p 20 s_def > /dev/null
  ./run_cfg.sh $p 20 s_rf ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1 > /dev/null
  for c in s_def s_rf; do
    f=logs/${p}_${c}_1.log
    ext=$(grep -m1 "clique extension" $f | sed 's/MIP: clique extension: //;s/ (.*//')
    lp=$(grep -m1 "root LP -\?[0-9]" $f | sed 's/MIP: root LP //;s/ (.*//')
    echo -e "$p\t$c\t$lp\t$ext\t$(tail -1 $f | cut -c1-160)"
  done
done
