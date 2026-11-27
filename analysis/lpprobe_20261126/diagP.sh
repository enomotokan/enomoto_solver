# 診断 P: P の 58 問 × 1・4 スレッド、種 1、60 秒、木に入ったところで止める (既定のみ)
PS=$(cd ../cbcimpl_20261029 && python -c "from probs import PROBS; print(' '.join(PROBS))")
for p in $PS; do
  for th in 1 4; do
    DIAG_STOP_TREE=1 python diag.py res_diagP.jsonl $p 1 $th 60
  done
done
