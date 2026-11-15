#!/bin/bash
# OBJPROBE の誤答の確かめ。cand1114 の fuzz.sh と同じ組を、CAND1114 + ENOMOTO_MIP_OBJPROBE=rpc (rpc) と、
# 同じく条件を外して毎ラウンド必ず回す形 (agg: ギャップの上限なし・乱択丸めは暫定解があっても) の 2 通りで回す。
# cargo test も同じ設定で。KS="rpch aggh" で h (軽いヒューリスティクスの束) の 2 通り (rpch・条件を外した aggh)。ログは fuzz_logs/<設定>_*.log。リポジトリの根から: bash analysis/objprobe_20261113/fuzz.sh
set -u
cd "$(dirname "$0")/../.."
L=analysis/objprobe_20261113/fuzz_logs; mkdir -p $L
BASE=$(python3 -c "import sys; sys.path.insert(0, 'analysis/cand1114_20261113'); from cfg import CAND1114; print(' '.join(f'{k}={v}' for k, v in CAND1114.items()))")
declare -A EX
EX[rpc]="ENOMOTO_MIP_OBJPROBE=rpc"
EX[rpch]="ENOMOTO_MIP_OBJPROBE=rpch"
EX[aggh]="ENOMOTO_MIP_OBJPROBE=rpch ENOMOTO_T_MIP_OBJPROBE_GAP=1e30 ENOMOTO_T_MIP_OBJPROBE_RR_GAP=-1 ENOMOTO_T_MIP_OBJPROBE_RR_ROUNDS=1000 ENOMOTO_T_MIP_OBJPROBE_GIVEUP=100000 ENOMOTO_T_MIP_OBJPROBE_H_MOVE=-1 ENOMOTO_T_MIP_OBJPROBE_H_FRAC=1e9"
EX[agg]="ENOMOTO_MIP_OBJPROBE=rpc ENOMOTO_T_MIP_OBJPROBE_GAP=1e30 ENOMOTO_T_MIP_OBJPROBE_RR_GAP=-1 ENOMOTO_T_MIP_OBJPROBE_RR_ROUNDS=1000 ENOMOTO_T_MIP_OBJPROBE_GIVEUP=100000"
run() { name=$1; shift; echo "== $name: $*"; env $ENVS "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
KS=${KS:-rpc agg}
for k in $KS; do
  ENVS="$BASE ${EX[$k]}"
  run ${k}_fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
  run ${k}_fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
  run ${k}_fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
  run ${k}_fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
done
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
for k in $KS; do
  echo "== cargo test (CAND1114 + $k)"; env $BASE ${EX[$k]} $CT > $L/cargo_$k.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_$k.log
done
