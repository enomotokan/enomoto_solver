#!/bin/bash
# 引き継いだ重みと厳密な重みの比較 (書き出した LP) と、physiciansched6-2 (E2C) での CUT_DSE_EXACT / ROWS_DSE_EXACT+LP_RESUME の 60 秒
cd "$(dirname "$0")"; D=$(pwd)
cd ../.. && maturin build --release -o dist 2>&1 | grep -E "^error|Built" && pip install --no-deps --force-reinstall dist/*.whl 2>&1 | grep -i "^success"
export RUSTFLAGS="-L /usr/lib/x86_64-linux-gnu -lpython3.13" LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
cargo test --release --lib cut_lp_debug --no-run 2>&1 | grep -E "^error|Executable"
cd "$D"
for P in dumps/run8/cutlp_r8_TimeLimit dumps/run7/cutlp_r8_TimeLimit dumps/run5/cutlp_r10_TimeLimit dumps/run5/cutlp_r0_Optimal dumps/run5/cutlp_r5_Optimal; do
  ./eno_dump.sh $P cmp_$(echo $P | tr '/' '_') CUTLP_ITERS=1 CUTLP_DSE_COMPARE=1 > /dev/null
  echo "== $P"; grep "CUTLP: inherited\|CUTLP: largest" logs/cmp_$(echo $P | tr '/' '_').log
done
E="ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_LP=1"
run() { name=$1; shift; env $E "$@" python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_$name.log 2>&1; echo "$name: $(tail -1 logs/physiciansched6-2_$name.log | cut -c1-120)"; grep "MIP: cut round .*: LP \|MIP: cut round .*obj\|continued after\|keeping the basis" logs/physiciansched6-2_$name.log | sed 's/MIP: cut round/  round/' | cut -c1-130; }
mkdir -p dumps/run10 dumps/run11
run e2c_cutexact_10 ENOMOTO_MIP_CUT_DSE_EXACT=1 ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=dumps/run10
run e2c_rowsexact_resume_11 ENOMOTO_MIP_ROWS_DSE_EXACT=1 ENOMOTO_MIP_LP_RESUME=1 ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=dumps/run11
