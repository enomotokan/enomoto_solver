#!/bin/bash
# P2-INF: cfx2 の部品の切り分け。問題:種 ごとに {def, lx, hc, fl, lxfl, hcfl} を 4 スレッド・600 秒で 1 本ずつ回す → logs/ablcfx2.txt
# lx = LIT_EXPLAIN、hc = HiGHS 式の衝突スコア、fl = 衝突の重み FLAT 0.1。リポジトリの根から: PYTHONPATH=<ビルド> bash analysis/p2inf_20261119/abl_cfx2.sh 問題:種 ...
cd "$(dirname "$0")/../.."
O=analysis/p2inf_20261119/logs/ablcfx2.txt
LX=ENOMOTO_MIP_LIT_EXPLAIN=1; HC=ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1; FL=ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT=0.1
for spec in "$@"; do IFS=: read p s <<< "$spec"
  f=$(python3 -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
  for c in def lx hc fl lxfl hcfl; do
    case $c in def) E="X=1";; lx) E="$LX";; hc) E="$HC";; fl) E="$FL";; lxfl) E="$LX $FL";; hcfl) E="$HC $FL";; esac
    r=$(env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit 600 --threads 4 2>/dev/null | grep '^{' | python3 -c 'import sys,json;r=json.loads(sys.stdin.read());print(r["status"],round(r["time"],1),r["nodes"])')
    echo "$p s$s $c $r" | tee -a $O
  done
done
