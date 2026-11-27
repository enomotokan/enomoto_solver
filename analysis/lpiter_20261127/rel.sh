#!/bin/bash
# 根の LP (整数を外した LP 緩和、.lpgap_cache/<問題>_rel.mps、analysis/lpgap_20261126/mklp.py で作る) を
# 当方 (ENOMOTO_PROF_PHASES_EXT つき) と HiGHS (前処理なし・1 スレッド・解析の段階 8 の計時つきと計時なし) で解く。
# 出力: rel.jsonl (1 行 1 回) と logs/ours_<LP>.out。リポジトリの根から: bash analysis/lpiter_20261127/rel.sh uccase12_rel ...
cd "$(dirname "$0")/../.."
D=analysis/lpiter_20261127; mkdir -p $D/logs
for lp in "$@"; do
  ENOMOTO_PROF_PHASES_EXT=1 python analysis/lpgap_20261126/one.py ours $lp 600 > $D/logs/ours_$lp.out 2>&1
  tail -1 $D/logs/ours_$lp.out >> $D/rel.jsonl
  for t in "" timer=0; do
    python -I $D/hphase.py rel .lpgap_cache/$lp.mps 600 $t | tail -1 | python -c "import json,sys; r=json.loads(sys.stdin.read()); r.update(who='highs', lp='$lp'); print(json.dumps(r))" >> $D/rel.jsonl
  done
  tail -3 $D/rel.jsonl | cut -c1-200
done
