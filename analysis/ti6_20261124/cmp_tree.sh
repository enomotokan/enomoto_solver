#!/bin/bash
# 試作 ENOMOTO_MIP_CUTOFF_USER_TREE の比べ (1 スレッド・種 1)。リポジトリの根から: bash analysis/ti6_20261124/cmp_tree.sh
# 結果は res_tree.jsonl (1 行 1 回)。打ち切り値なしの P の 5 問 (120 秒) と、traininstance6 の打ち切り値つき (600 秒)
cd "$(dirname "$0")/../.."
D=analysis/ti6_20261124; out=$D/res_tree.jsonl
one() { p=$1; tl=$2; tag=$3; shift 3
  r=$(env "$@" ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads 1 2>/dev/null | grep '^{' | tail -1)
  echo "{\"problem\": \"$p\", \"tag\": \"$tag\", \"env\": \"$*\", \"result\": ${r:-null}}" | tee -a $out | cut -c1-230; }
for p in misc07 binkar10_1 mas76 pk1 neos-911970; do
  one $p 120 def X=1
  one $p 120 tree ENOMOTO_MIP_CUTOFF_USER_TREE=1
done
one traininstance6 600 cut_def ENOMOTO_MIP_CUTOFF=28290.02829
one traininstance6 600 cut_tree ENOMOTO_MIP_CUTOFF=28290.02829 ENOMOTO_MIP_CUTOFF_USER_TREE=1
