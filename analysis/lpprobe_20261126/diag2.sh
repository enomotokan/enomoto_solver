# 診断 2: P で LP プロービングが固定を出した 5 問 × 1・4 スレッド × 種 1〜3 × {既定, 子の試しなし (PASSES=0)}、60 秒
for p in neos-3083819-nubu neos-860300 flugpl dcmulti blend2; do
  for s in 1 2 3; do for th in 1 4; do
    python diag.py res_diag2.jsonl $p $s $th 60
    python diag.py res_diag2.jsonl $p $s $th 60 ENOMOTO_T_MIP_LP_PROBE_PASSES=0
  done; done
done
