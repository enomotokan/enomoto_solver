#!/bin/bash
# P2-INF: cfx の部品の切り分け。問題・種ごとに {def, cfx, lx だけ, 重み+HiGHS 式だけ, cfx から HiGHS 式を抜く} を 4 スレッドで 1 本ずつ回す。
# リポジトリの根から: PYTHONPATH=<ビルド> bash analysis/p2inf_20261119/abl_cfx.sh 問題:種:秒 ... → logs/ablcfx.txt
cd "$(dirname "$0")/../.."
O=analysis/p2inf_20261119/logs/ablcfx.txt
LX=ENOMOTO_MIP_LIT_EXPLAIN=1; HC=ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1; CW=ENOMOTO_T_MIP_CONFLICT_WEIGHT=0.1
for spec in "$@"; do IFS=: read p s tl <<< "$spec"
  f=$(python3 -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
  for c in def cfx lx cwhc lxcw; do
    case $c in def) E="X=1";; cfx) E="$LX $HC $CW";; lx) E="$LX";; cwhc) E="$HC $CW";; lxcw) E="$LX $CW";; esac
    r=$(env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit $tl --threads 4 2>/dev/null | grep '^{' | python3 -c 'import sys,json;r=json.loads(sys.stdin.read());print(r["status"],round(r["time"],1),r["nodes"])')
    echo "$p s$s $c $r" | tee -a $O
  done
done
