# 木の中の再スタートの試作 `ENOMOTO_MIP_TREE_RESTART_INC2` の対の比べ (2026-11-17、treeclose 9.63 節)

既定の木の中の再スタートは刈った重みの伸びで判定し、暫定解が良くなって被約費用で多くの列が固まることを見ない (tbfp-network: HiGHS は最適解の後に
4 回再スタートして 1 節点で閉じ、当方は木の中で最適解を見つけても再スタートせず 3201 節点)。試作 INC2 は、暫定解が良くなり根の被約費用で新たに
固まった整数列が、根の始めに自由だった整数列の `ENOMOTO_T_MIP_TREE_RESTART_INC2_FRAC` (既定 0.1) 以上なら木の中で再スタートする
(既定の判定に足す。ノード数の上限なし。残り時間が時間上限の 1 割未満なら見送る)。

設定 (cfg.py): def (既定)・inc2。

- P: 58 問 × 種 1〜3 × 2 設定、4 スレッド・60 秒 (348 回)、6 分担 (見積り各約 29 分): `python run.py P K` (K = 0〜5)
- M: tbfp-network・irp・neos-3627168-kasai・supportcase33・unitcal_7・rococoC10-001000 × 種 1・2 × 2 設定、4 スレッド・600 秒 (24 回)、
  6 分担 (各 4 回、最大約 41 分): `python run.py M K` (K = 0〜5)
- **534261f49 以降でビルドして回す。** 集計 `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt`。
