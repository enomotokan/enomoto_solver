#!/bin/bash
# LexRed の正しさの fuzz: 設定 (環境変数) を引数で受け、対称性の fuzz 群を回して bad の行だけ出す。
# リポジトリの根から: bash analysis/lexred_20261118/fuzz_lex.sh 名前 [KEY=VAL ...]  → analysis/lexred_20261118/fuzz_名前.txt
cd "$(dirname "$0")/../.."
name=$1; shift
O=analysis/lexred_20261118/fuzz_$name.txt; : > $O
H="ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_NO_CUTS=1 ENOMOTO_MIP_NO_RESTART=1"
run() { tag=$1; shift; r=$(env ENOMOTO_MIP_REL_GAP=1e-9 "$@" 2>/dev/null | grep -E "^bad|BAD" | tr '\n' ' '); echo "$tag: $r" >> $O; }
for t in 1 4; do
  run "of_noorb_nocut_t$t" $H ENOMOTO_MIP_THREADS=$t "$@" python analysis/sym_of_20261028/fuzz_of.py 1000 1000 2
  run "of_noorb_t$t" ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_THREADS=$t "$@" python analysis/sym_of_20261028/fuzz_of.py 500 3000 2
  run "of_def_t$t" ENOMOTO_MIP_THREADS=$t "$@" python analysis/sym_of_20261028/fuzz_of.py 500 5000 2
  run "symdyn_hard_t$t" $H ENOMOTO_MIP_THREADS=$t "$@" python analysis/symdyn_20261110/fuzz_symdyn.py 400 0 2
  run "symdyn_def_t$t" ENOMOTO_MIP_THREADS=$t "$@" python analysis/symdyn_20261110/fuzz_symdyn.py 400 1000 1
  run "symm_noorb_t$t" ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_THREADS=$t "$@" python scripts/mip_fuzz_symmetric.py 300 0 1
done
cat $O
