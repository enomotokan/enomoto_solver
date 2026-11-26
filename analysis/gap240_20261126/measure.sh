#!/bin/sh
# 足りない数 (根の下界・前処理の後の大きさ・ノードあたりの時間・最初の解) を取る回。ソルバーは同時に 1 つ、順に回す。途中から再開できる (ログが終わっていれば飛ばす)。
# 使い方: sh measure.sh   (ビルドは c8e468e25 の worktree で pip install -e . したもの。計測中はビルドしない)
#  1. SCIP 10.0.2 (pyscipopt 6.3.0、逐次、種 1、600 秒) の既定をログ・統計つきで: scipab.py (rootgap_20261121 の写し) → logs/scip_<問題>_def.{log,stat}、scipab.jsonl
#  2. HiGHS 1.15.1 (4 スレッド、600 秒) をログつきで: logs/highs_<問題>_t4.log (最後の行が結果の JSON)
#  3. 当方 (1 スレッド、種 1、600 秒、ENOMOTO_MIP_LOG=1): logs/ours_<問題>_t1.log。combo2 と同じ読み込みの道 (run_miplib_benchmark.py --worker-ours) で、最後の行が結果の JSON
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
SCIP_P=${SCIP_P:-"30n20b8 CMS750_4 bnatt400 dano3_5 istanbul-no-cutoff mcsched rocI-4-11 tbfp-network fhnw-binpack4-4 swath1"}
HIGHS_P=${HIGHS_P:-"csched008 neos-873061 net12 satellites2-40 satellites2-60-fs sct2 uccase12 tbfp-network"}
OURS_P=${OURS_P:-"csched008 net12 satellites2-40 sct2 uccase12 neos-873061 30n20b8 bnatt400 istanbul-no-cutoff mcsched"}
for p in $SCIP_P; do
  [ -s "logs/scip_${p}_def.stat" ] && continue
  python scipab.py "$p" def --tl 600 --seed 1 --log
done
for p in $HIGHS_P; do
  grep -q '^{' "logs/highs_${p}_t4.log" 2>/dev/null && continue
  ENOMOTO_BENCH_HIGHS_LOG=1 python "$R/scripts/run_miplib_benchmark.py" --worker-highs "$R/.miplib_cache/$p.mps" --time-limit 600 --threads 4 > "logs/highs_${p}_t4.log" 2>&1
  echo "highs $p $(tail -1 logs/highs_${p}_t4.log)"
done
for p in $OURS_P; do
  grep -q '^{' "logs/ours_${p}_t1.log" 2>/dev/null && continue
  ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python "$R/scripts/run_miplib_benchmark.py" --worker-ours "$R/.miplib_cache/$p.mps" --time-limit 600 --threads 1 > "logs/ours_${p}_t1.log" 2>&1
  echo "ours $p $(tail -1 logs/ours_${p}_t1.log)"
done
echo 全部済み
