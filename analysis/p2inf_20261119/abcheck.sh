#!/bin/bash
# P2-INF 7 節: 分枝のスコアの係数を兄弟の部分木に 2 値で入れ切りした対 (ENOMOTO_MIP_CWM=ab) と、同じ係数を全体で 2 値にした求解 (1 スレッド・種 1)。
# 1 行 1 回: 問題 係数 種類 (ab / a / b) → logs/abcheck.txt。リポジトリの根から: PYTHONPATH=<ビルド> bash analysis/p2inf_20261119/abcheck.sh 秒 < 並び
cd "$(dirname "$0")/../.."
tl=$1
one() {
  p=$1; c=$2; k=$3
  case $c in inf) a=0; b=0.001; g=ENOMOTO_T_MIP_HY_INF_W;; cut) a=0; b=0.001; g=ENOMOTO_T_MIP_HY_CUT_W;; dynw) a=0.1; b=10; g=ENOMOTO_T_MIP_HY_DYNW_MULT;;
    psfloor) a=0.000001; b=1; g=ENOMOTO_T_MIP_HY_PS_FLOOR;; minrel) a=1; b=16; g=ENOMOTO_T_MIP_MINREL;; esac
  case $k in ab) E="ENOMOTO_MIP_CWM=ab ENOMOTO_MIP_CWM_AB=$c:$a:$b";; a) E="$g=$a";; b) E="$g=$b";; esac
  f=.miplib_cache/$p.mps
  log=analysis/p2inf_20261119/logs/ab_${p}_${c}_$k.log
  env $E ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit $tl --threads 1 > $log 2>&1
  r=$(grep '^{' $log | python3 -c 'import sys,json;r=json.loads(sys.stdin.read());print(r["status"],round(r["time"],1),r["nodes"])')
  y=$(grep "CWM final" $log | tail -1 | grep -o "observations [0-9]*, mean y = ln(size+/size-) [-+0-9.]* (se [0-9.na]*)" | sed 's/observations \([0-9]*\), mean y = ln(size+\/size-) \([-+0-9.]*\) (se \([0-9.na]*\))/pairs=\1 lnb_a=\2 se=\3/')
  echo "$p $c $k $r $y" >> analysis/p2inf_20261119/logs/abcheck.txt
}
export -f one; export tl
xargs -P 4 -L 1 bash -c 'one $0 $1 $2'
