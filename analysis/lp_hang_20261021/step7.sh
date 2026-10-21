#!/bin/bash
# 修正 + LP_RESUME (+ROWS_DSE_EXACT) の E2C と、木のノードでの重みのずれの確認 (DSE_CHECK、修正 vs 以前)
cd "$(dirname "$0")"
E="ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_LP=1"
run() { name=$1; shift; env $E "$@" python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_$name.log 2>&1; echo "$name: $(tail -1 logs/physiciansched6-2_$name.log | cut -c1-140)"; grep "MIP: cut round .*: .* cuts (of\|continued after\|removed the cuts" logs/physiciansched6-2_$name.log | sed 's/MIP: cut round/  round/' | cut -c1-120; }
run e2c_fix2_resume_18 ENOMOTO_MIP_LP_RESUME=1
run e2c_fix2_resume_rows_19 ENOMOTO_MIP_LP_RESUME=1 ENOMOTO_MIP_ROWS_DSE_EXACT=1
for p in 10teams binkar10_1; do
  for cfg in "fix2" "nopolishdse ENOMOTO_NO_POLISH_DSE=1"; do
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
echo "== step7 done"
