#!/bin/sh
# 誤答の検査 (SUBCAP 改良、2026-11-08): 設定ごとに fuzz_tiny.py 400・mip_fuzz_vs_highs.py 300 (並列のサブ MIP が出るように 4 スレッド)
cd "$(dirname "$0")/../../.."
F=analysis/submip_cap_20261106/fuzz
for cfg in def JT BT; do
  case $cfg in
    def) E="";;
    *) E="ENOMOTO_MIP_SUBCAP=$cfg";;
  esac
  echo "== $cfg tiny" >> $F/fuzz.log; env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 >> $F/fuzz.log 2>/dev/null
  echo "== $cfg vs_highs" >> $F/fuzz.log; env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300 >> $F/fuzz.log 2>/dev/null
done
echo FUZZ_DONE >> $F/fuzz.log
