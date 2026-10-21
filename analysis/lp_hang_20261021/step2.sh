#!/bin/bash
# 修正の試作のビルドと、physiciansched6-2 の 60 秒 (既定の重み / ENOMOTO_MIP_ROWS_DSE_EXECT) と、書き出した LP での重みの実験
cd "$(dirname "$0")"; D=$(pwd)
cd ../.. && maturin build --release -o dist 2>&1 | grep -E "^error|Built" && pip install --no-deps --force-reinstall dist/*.whl 2>&1 | grep -i "success"
export RUSTFLAGS="-L /usr/lib/x86_64-linux-gnu -lpython3.13" LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
cargo test --release --lib cut_lp_debug --no-run 2>&1 | grep -E "^error|Executable"
cd "$D"
E="ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_LP=1"
rm -rf dumps/run5 dumps/run6; mkdir -p dumps/run5 dumps/run6
env $E ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=dumps/run5 python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_e2c_dump_5.log 2>&1
echo "run5 (default): $(tail -1 logs/physiciansched6-2_e2c_dump_5.log)"
grep "MIP: cut round .*: LP " logs/physiciansched6-2_e2c_dump_5.log | cut -c1-110
env $E ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=dumps/run6 ENOMOTO_MIP_ROWS_DSE_EXACT=1 python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_e2c_dsex_6.log 2>&1
echo "run6 (ROWS_DSE_EXACT): $(tail -1 logs/physiciansched6-2_e2c_dsex_6.log)"
grep "MIP: cut round .*: LP \|cut round .*obj" logs/physiciansched6-2_e2c_dsex_6.log | cut -c1-150
P=$(ls dumps/run5/*TimeLimit.mps 2>/dev/null | head -1); P=${P%.mps}
echo "harness on $P"
if [ -n "$P" ]; then
  ./eno_dump.sh $P eno_run5_unit CUTLP_TIME=120
  ./eno_dump.sh $P eno_run5_inherit CUTLP_TIME=120 CUTLP_DSE=1 ENOMOTO_WARM_DSE=1
  ./eno_dump.sh $P eno_run5_inherit_newexact CUTLP_TIME=120 CUTLP_DSE=1 CUTLP_DSE_NEWEXACT=1 ENOMOTO_WARM_DSE=1
  ./eno_dump.sh $P eno_run5_exact CUTLP_TIME=120 ENOMOTO_DSE_EXACT_INIT=1
  python3 solve_dump.py $P 120 warm dual > logs/highs_run5_timelimit.log 2>&1; grep RESULT logs/highs_run5_timelimit.log
fi
