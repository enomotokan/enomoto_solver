#!/bin/bash
# 検証の一括実行 (ソルバーは同時に 1 つ)
cd "$(dirname "$0")"
B=../equiv2_bug_20261022
echo "== verify1"; python3 verify1.py | sed "s/ENOMOTO-Solver.*Result:/... Result:/"
echo "== uccase12 rep (COEF_TIGHTEN_TINY=1)"
python3 $B/rep.py uc_rep 4 45 4 ENOMOTO_MIP_COEF_TIGHTEN_TINY=1
export ENOMOTO_MIP_REL_GAP=1e-9
echo "== fuzz default"; python3 ../../scripts/mip_fuzz_vs_highs.py 300 2>/dev/null | grep -E "^seed|^bad"
echo "== fuzz eq2r"; ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_PROBE_EQUIV2_RESTART=1 python3 ../../scripts/mip_fuzz_vs_highs.py 300 2>/dev/null | grep -E "^seed|^bad"
for cfg in "default:" "tinyonly:ENOMOTO_MIP_COEF_TIGHTEN_TINY=1" "eq2r:ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_PROBE_EQUIV2_RESTART=1" "old:ENOMOTO_MIP_NO_TINY_WIDTH_FIX=1"; do
  n=${cfg%%:*}; e=${cfg#*:}
  echo "== fuzz_tiny $n"; env $e python3 $B/fuzz_tiny.py 300 2>/dev/null | grep -E "^seed|^bad"
  echo "== fuzz_tiny_cls $n"; env $e python3 $B/fuzz_tiny_cls.py 0 300 2>/dev/null | tail -1
  echo "== fuzz_mixtiny $n"; env $e python3 fuzz_mixtiny.py 300 2>/dev/null | grep -E "^seed|^bad"
done
echo "== check55"; python3 check55.py check55.jsonl > check55.txt 2>&1; tail -2 check55.txt
echo "== done"
