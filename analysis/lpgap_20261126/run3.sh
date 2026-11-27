#!/bin/bash
# P と根の LP の重い問題の LP 緩和を、当方 (内訳つき) と HiGHS (前処理なし・あり) で解く。出力は res_rel.jsonl と ours_<LP>.out
cd "$(dirname "$0")/../.."
D=analysis/lpgap_20261126
for lp in "$@"; do
  ENOMOTO_PROF_PHASES_EXT=1 python $D/one.py ours $lp 300 > $D/ours_$lp.out 2>&1; tail -1 $D/ours_$lp.out >> $D/res_rel.jsonl
  python $D/one.py highs $lp 300 2>/dev/null | tail -1 >> $D/res_rel.jsonl
  python $D/one.py highs_pre $lp 300 2>/dev/null | tail -1 >> $D/res_rel.jsonl
done
