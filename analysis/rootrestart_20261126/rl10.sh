#!/bin/sh
# 10 節: RSTLOOSE の数問の確かめ。1 スレッド・種 1。HiGHS 型 3 問は 600 秒、ほかは 60 秒。ソルバーは同時に 1 つ。
# 設定: def (変数なし)、t (閾値をそろえる)、tpci (t + 持ち越し 3 つ)、am / amt (p200x1188c の型: ROOTRESTART2=am に t を足すと直るか)
# 使い方 (リポジトリの根から): sh analysis/rootrestart_20261126/rl10.sh → logs/RL_<問題>_<設定>.log、rl10.jsonl。途中から再開できる
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
mkdir -p logs
run() { p=$1; name=$2; tl=$3; shift 3
  grep -q '^{' "logs/RL_${p}_${name}.log" 2>/dev/null && return
  env -u ENOMOTO_MIP_RSTLOOSE -u ENOMOTO_MIP_ROOTRESTART2 "$@" ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python "$R/scripts/run_miplib_benchmark.py" --worker-ours "$R/.miplib_cache/$p.mps" --time-limit $tl --threads 1 > "logs/RL_${p}_${name}.log" 2>&1
  echo "{\"problem\": \"$p\", \"cfg\": \"$name\", \"restarts\": $(grep -c '^MIP: restart [0-9]* (' logs/RL_${p}_${name}.log), \"result\": $(grep '^{' logs/RL_${p}_${name}.log | tail -1)}" >> rl10.jsonl
}
for p in p200x1188c drayage-25-23 h80x6320d gt2 p0201 binkar10_1 qnet1; do
  run $p def 60 ENOMOTO_DUMMY=1
  run $p t 60 ENOMOTO_MIP_RSTLOOSE=t
  run $p tpci 60 ENOMOTO_MIP_RSTLOOSE=tpci
done
for p in p200x1188c; do
  run $p am 60 ENOMOTO_MIP_ROOTRESTART2=am
  run $p amt 60 ENOMOTO_MIP_ROOTRESTART2=am ENOMOTO_MIP_RSTLOOSE=t
done
for p in satellites2-40 satellites2-60-fs uccase12; do
  run $p def 600 ENOMOTO_DUMMY=1
  run $p t 600 ENOMOTO_MIP_RSTLOOSE=t
  run $p tpci 600 ENOMOTO_MIP_RSTLOOSE=tpci
done
echo 全部済み
