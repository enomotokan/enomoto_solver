#!/bin/sh
# 診断 1 (analysis/rootrestart_20261126 の 2 節): 根の再スタートの判定の時点で、被約費用で固定できる整数列の数を、今の打ち切り値と既知の最適値で数える
# (ENOMOTO_MIP_RR_DIAG=<最適値>、RRDIAG の行)。既定と ACFIX=d (解析的中心の双対で固定) の 2 つ。1 スレッド・種 1・600 秒。ソルバーは同時に 1 つ。
# 使い方 (リポジトリの根から): sh analysis/rootrestart_20261126/diag.sh   → logs/<問題>_<設定>.log (コミットするのは要約だけ)
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
run() { p=$1; u=$2; name=$3; shift 3
  grep -q '^{' "logs/${p}_${name}.log" 2>/dev/null && return
  env "$@" ENOMOTO_MIP_RR_DIAG=$u ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python "$R/scripts/run_miplib_benchmark.py" --worker-ours "$R/.miplib_cache/$p.mps" --time-limit 600 --threads 1 > "logs/${p}_${name}.log" 2>&1
  echo "$p $name $(grep -c RRDIAG logs/${p}_${name}.log) $(tail -1 logs/${p}_${name}.log)"
}
for spec in "uccase12 11507.4050616" "satellites2-60-fs -19" "satellites2-40 -19" "sct2 -230.9891623" "csched008 173" "neos-873061 113.6562385063"; do
  set -- $spec
  run $1 $2 def ENOMOTO_DUMMY=1
  run $1 $2 acd ENOMOTO_MIP_HO4_ACFIX=d
done
echo 全部済み
