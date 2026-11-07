#!/bin/sh
# 誤答の検査 (2026-11-07): 試作の設定ごとに fuzz_tiny.py 400・mip_fuzz_vs_highs.py 300・fuzz_snf.py 1000 (既定も比べに)
cd "$(dirname "$0")/../../.."
F=analysis/snfmodk_20261107/fuzz
for cfg in def snf snfcv; do
  case $cfg in
    def) E="";;
    snf) E="ENOMOTO_MIP_SNFMODK=c";;
    snfcv) E="ENOMOTO_MIP_SNFMODK=cvk ENOMOTO_MIP_MODKVB=1";;
  esac
  echo "== $cfg tiny" >> $F/fuzz.log; env $E ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 >> $F/fuzz.log 2>/dev/null
  echo "== $cfg vs_highs" >> $F/fuzz.log; env $E ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300 >> $F/fuzz.log 2>/dev/null
  echo "== $cfg fuzz_snf" >> $F/fuzz.log; env $E ENOMOTO_MIP_REL_GAP=1e-9 python $F/fuzz_snf.py 1000 >> $F/fuzz.log 2>/dev/null
done
echo FUZZ_DONE >> $F/fuzz.log
