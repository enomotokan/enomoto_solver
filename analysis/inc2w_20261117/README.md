# INC2 に仕事量の条件を足した形 (inc2w) の対の比べ (2026-11-17、treeclose 9.67 節)

`inc2_20261117` と同じ組 (P 58 問 × 種 1〜3・60 秒、M 6 問 × 種 1・2・600 秒)。inc2w は INC2 に、木に入ってからの主のスレッドの手間 (`crate::ticks`)
が根の段の手間 (木に入った時点の累計、前の再スタートの分も含む) の `ENOMOTO_T_MIP_TREE_RESTART_INC2_WORKMULT` (既定 1) 倍以上のときだけ使う条件を足した形。

- P: `python run.py P K` (K = 0〜5)、M: `python run.py M K` (K = 0〜5)。**この台本のコミット以降でビルドして回す。**
- 集計 `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt`。
