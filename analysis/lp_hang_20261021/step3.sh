#!/bin/bash
# 重みの書き出し精度を 17 桁にして run6 の設定をやり直し (run7)、書き出した LP で経路の敏感さ (重みの揺らぎ) を見る
cd "$(dirname "$0")"; D=$(pwd)
cd ../.. && maturin build --release -o dist 2>&1 | grep -E "^error|Built" && pip install --no-deps --force-reinstall dist/*.whl 2>&1 | grep -i "success"
export RUSTFLAGS="-L /usr/lib/x86_64-linux-gnu -lpython3.13" LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
cargo test --release --lib cut_lp_debug --no-run 2>&1 | grep -E "^error|Executable"
cd "$D"
E="ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_LP=1"
rm -rf dumps/run7; mkdir -p dumps/run7
env $E ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=dumps/run7 ENOMOTO_MIP_ROWS_DSE_EXACT=1 python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_e2c_dsex_7.log 2>&1
echo "run7 (ROWS_DSE_EXACT, 17-digit weights): $(tail -1 logs/physiciansched6-2_e2c_dsex_7.log)"
grep "MIP: cut round .*: LP " logs/physiciansched6-2_e2c_dsex_7.log | cut -c1-110
P=$(ls dumps/run7/*TimeLimit.mps 2>/dev/null | head -1); P=${P%.mps}
echo "harness on $P"
if [ -n "$P" ]; then
  ./eno_dump.sh $P eno_run7_inherit CUTLP_TIME=120 CUTLP_DSE=1 ENOMOTO_WARM_DSE=1
  for s in 1 2 3; do ./eno_dump.sh $P eno_run7_jitter$s CUTLP_TIME=120 CUTLP_DSE=1 ENOMOTO_WARM_DSE=1 CUTLP_DSE_JITTER=1e-9 CUTLP_SEED=$s; done
  python3 solve_dump.py $P 120 warm dual > logs/highs_run7_timelimit.log 2>&1; grep RESULT logs/highs_run7_timelimit.log
fi
for s in 1 2 3; do ./eno_dump.sh dumps/run6/cutlp_r8_TimeLimit eno_run6r8_jitter$s CUTLP_TIME=120 CUTLP_DSE=1 ENOMOTO_WARM_DSE=1 CUTLP_DSE_JITTER=1e-9 CUTLP_SEED=$s; done
