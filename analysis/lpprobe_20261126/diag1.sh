# 診断 1: 前処理の重い 10 問 × 1・4 スレッド × {既定, 子の試しなし (PASSES=0)}、種 1、60 秒
for p in cod105 dano3_3 mzzv42z physiciansched6-2 nw04 neos-827175 graph20-20-1rand neos-860300 30n20b8 supportcase7; do
  for th in 4 1; do
    python diag.py res_diag1.jsonl $p 1 $th 60
    python diag.py res_diag1.jsonl $p 1 $th 60 ENOMOTO_T_MIP_LP_PROBE_PASSES=0
  done
done
