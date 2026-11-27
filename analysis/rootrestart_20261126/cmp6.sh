#!/bin/sh
# 6 問の比べ (README の 5 節): 既定と ROOTRESTART2=eam を、1 スレッド・種 1・600 秒で。ビルドは 53b2ade2e。ソルバーは同時に 1 つ。
# 使い方 (リポジトリの根から): sh analysis/rootrestart_20261126/cmp6.sh → logs/<問題>_c<設定>.log、cmp6.jsonl
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
run() { p=$1; name=$2; shift 2
  grep -q '^{' "logs/${p}_c${name}.log" 2>/dev/null && return
  env "$@" ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python "$R/scripts/run_miplib_benchmark.py" --worker-ours "$R/.miplib_cache/$p.mps" --time-limit 600 --threads 1 > "logs/${p}_c${name}.log" 2>&1
  echo "{\"problem\": \"$p\", \"cfg\": \"$name\", \"restarts\": $(grep -c 'MIP: early restart\|TREE restart\|ROOTRESTART2 .*restart true' logs/${p}_c${name}.log), \"result\": $(tail -1 logs/${p}_c${name}.log)}" >> cmp6.jsonl
}
for p in satellites2-60-fs satellites2-40 uccase12 sct2 csched008 neos-873061; do
  run $p def ENOMOTO_DUMMY=1
  run $p eam ENOMOTO_MIP_ROOTRESTART2=eam
done
# eam2: 初回の再スタートの手間の見積もりを「前処理 + 根の LP」に直したビルドで、見積もりの行が出た 3 問をもう一度 (設定は eam と同じ)
for p in satellites2-60-fs satellites2-40 uccase12; do
  run $p eam2 ENOMOTO_MIP_ROOTRESTART2=eam
done
echo 全部済み
