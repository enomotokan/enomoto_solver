#!/bin/bash
# ENOMOTO_DSE_WEIGHT_CHECK (選んだ行の重みの検査・選び直し・全行の計算し直し) の試作の確認
cd "$(dirname "$0")"
cd ../.. && maturin build --release -o dist 2>&1 | grep -E "^error|Built" && pip install --no-deps --force-reinstall dist/*.whl 2>&1 | grep -i "^success"; cd analysis/lp_hang_20261021
E="ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_LP=1 ENOMOTO_DEBUG_EXT_ITERS=1"
run() { name=$1; shift; env $E "$@" python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_$name.log 2>&1; echo "== $name: $(tail -1 logs/physiciansched6-2_$name.log | cut -c1-140)"; grep "MIP: cut round .*: .* cuts (of\|continued after\|removed the cuts" logs/physiciansched6-2_$name.log | sed 's/MIP: cut round/  round/' | cut -c1-120; grep "dse_weight_check" logs/physiciansched6-2_$name.log | head -14 | sed 's/DEBUG_EXT: //'; }
run e2c_old_wcheck_20 ENOMOTO_NO_POLISH_DSE=1 ENOMOTO_DSE_WEIGHT_CHECK=1
run e2c_old_wcheck_rejectonly_21 ENOMOTO_NO_POLISH_DSE=1 ENOMOTO_DSE_WEIGHT_CHECK=1 ENOMOTO_T_DSE_RECOMPUTE=0
run e2c_fix2_wcheck_22 ENOMOTO_DSE_WEIGHT_CHECK=1
for p in binkar10_1 10teams; do
  for cfg in "old_wcheck ENOMOTO_NO_POLISH_DSE=1 ENOMOTO_DSE_WEIGHT_CHECK=1" "fix2_wcheck ENOMOTO_DSE_WEIGHT_CHECK=1"; do
    set -- $cfg; name=$1; shift
    env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_DEBUG_DSE_CHECK=1 "$@" python3 eno_run.py ../../.miplib_cache/$p.mps 20 1 > logs/${p}_dsecheck_$name.log 2>&1
    python3 - $p $name logs/${p}_dsecheck_$name.log <<'PY'
import sys, re
p, name, f = sys.argv[1:4]
n = bad = big = 0; worst = 0.0
for line in open(f):
    m = re.search(r"DSE_CHECK after solve \((\d+) iters.*ratio >=10: (\d+), <=0.1: (\d+), max ([0-9.e+-]+)", line)
    if not m: continue
    it, over, under, mx = int(m.group(1)), int(m.group(2)), int(m.group(3)), float(m.group(4))
    n += 1; bad += (over > 0); big += (over >= 10); worst = max(worst, mx)
print(f"{p} {name}: solves checked {n}, with a row >=10x off {bad}, with >=10 such rows {big}, worst ratio {worst:.1e}")
PY
  done
done
run2() { p=$1; name=$2; shift 2; n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
  env ENOMOTO_MIP_THREADS=4 "$@" python3 eno_run.py ../../.miplib_cache/$p.mps 60 4 > logs/${p}_${name}_${n}.log 2>&1
  echo "$p $name $n $(tail -1 logs/${p}_${name}_${n}.log | cut -c1-150)"; }
for p in 10teams binkar10_1 neos-911970 misc07; do
  run2 $p fix2_wcheck ENOMOTO_MIP_LOG=1 ENOMOTO_DSE_WEIGHT_CHECK=1
  run2 $p old_wcheck ENOMOTO_MIP_LOG=1 ENOMOTO_NO_POLISH_DSE=1 ENOMOTO_DSE_WEIGHT_CHECK=1
done
echo "== step8 done"
