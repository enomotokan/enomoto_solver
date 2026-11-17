# LMBOOST の間に主を待たせない試作 `ENOMOTO_MIP_LMBOOST_ASYNC` の対の比べ (2026-11-17、treeclose 9.72 節)

LMBOOST (根のカットのループの後、暫定解が Local-MIP の解だけのとき Local-MIP のスレッドを足して回す) は、止める条件まで主を待たせる
(tbfp-network の 2・4 スレッドで 33〜50 秒)。試作は Local-MIP のスレッドを裏で回したまま主を根のヒューリスティクスへ進め、止める条件は主が解を受け取るたびに見て、
木に入るときに必ず止める。tbfp-network (4 スレッド・種 1、木に入ったら止める): 最初の再スタート 257 → 226 秒。

- P: 58 問 × 種 1〜3 × {def, lma}、4 スレッド・60 秒、6 分担: `python run.py P K` (K = 0〜5)
- M: tbfp-network・irp・mzzv11・neos-3627168-kasai・rococoC10-001000・supportcase33 × 種 1・2 × {def, lma}、600 秒、6 分担: `python run.py M K` (K = 0〜5)
- この試作のコミット以降でビルド。集計 `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt`。
