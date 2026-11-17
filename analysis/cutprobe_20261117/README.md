# カットの選び方の下見の試作 `ENOMOTO_MIP_CUTSEL_PROBE_AFFORD` の対の比べ (2026-11-17、treeclose 9.64 節)

下見 (既定と POOL の側のカットの選び方の比べ) は、1 ラウンドずつの比べで決まらなければ k (6) ラウンドの比べをするが、1 ラウンドの LP が重い問題では
予算 (時間上限の 5%) を大きく超える (tbfp-network: 予算 29.8 秒に 46 秒、k ラウンドの比べの既定の側 5 ラウンド 31.6 秒を使ったのに、POOL の側が
1 ラウンドで止まり 1 ラウンド目の値で比べていた)。試作 AFFORD は、1 ラウンド目の両側の時間 × k が残りの予算を超えるときは 1 ラウンド目の値で決める
(tbfp-network で下見 46 → 10 秒、選ぶ側は同じ POOL)。

- P: 58 問 × 種 1〜3 × {def, aff}、4 スレッド・60 秒、6 分担: `python run.py P K` (K = 0〜5)
- M: tbfp-network・irp・mzzv11・neos-3627168-kasai・rococoC10-001000・supportcase33 × 種 1・2 × {def, aff}、600 秒、6 分担: `python run.py M K`
- この試作のコミット以降でビルド。集計 `python summ.py P > summ_P.txt`、`python summ.py M > summ_M.txt`。
