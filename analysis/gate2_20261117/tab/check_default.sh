#!/bin/bash
# 既定で入れる形 (default_on.patch を当てたビルド、PYTHONPATH=$1) の確かめ: cargo test (作業木 $2)・fuzz (しきい値 0.01 と既定のしきい値)・p0201 40 種。nice 19。
set -u
P=$1; W=$2; R=/home/user/enomoto_solver; L=$R/analysis/gate2_20261117/tab/fuzz_logs; mkdir -p $L
cd $W && LD_LIBRARY_PATH=/usr/local/lib nice -n 19 cargo test --release --config 'target.x86_64-unknown-linux-gnu.rustflags=["-C","link-arg=-L/usr/local/lib","-C","link-arg=-lpython3.13"]' 2>&1 | grep -E "test result|FAILED" | head -3
cd $R
run() { name=$1; shift; echo "== $name"; env PYTHONPATH=$P nice -n 19 "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "bad|mismatches" $L/$name.log | tail -1; }
run d_tiny env ENOMOTO_MIP_REL_GAP=1e-9 ENOMOTO_T_MIP_TAB_SIG_W=0.01 ENOMOTO_T_MIP_TAB_SIG_CONT=2 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run d_highs env ENOMOTO_MIP_REL_GAP=1e-9 ENOMOTO_T_MIP_TAB_SIG_W=0.01 ENOMOTO_T_MIP_TAB_SIG_CONT=2 python3 scripts/mip_fuzz_vs_highs.py 300
run d_objint env ENOMOTO_T_MIP_TAB_SIG_W=0.01 ENOMOTO_T_MIP_TAB_SIG_CONT=2 python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run d_slack env ENOMOTO_T_MIP_TAB_SIG_W=0.01 ENOMOTO_T_MIP_TAB_SIG_CONT=2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run d_tiny_def env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run d_highs_def env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 150
echo "== p0201 40 種 (既定のしきい値)"; bad=0
for s in $(seq 1 40); do o=$(env PYTHONPATH=$P ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s nice -n 19 python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/p0201.mps --time-limit 60 --threads 4 2>/dev/null | tail -1 | python3 -c 'import json,sys;print(json.load(sys.stdin)["objective"])'); [ "$o" != "7615.0" ] && { bad=$((bad+1)); echo "  $s $o"; }; done; echo "p0201 bad=$bad/40"
echo CHECK_DONE
