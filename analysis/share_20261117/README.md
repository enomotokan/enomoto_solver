# 共有を切る・増やす診断 (treeclose 9.75 節の 3、2026-11-17)

4 スレッドで既定に診断の環境変数を 1 つずつ足して (`cfg.py` の 10 設定)、節点の水増しと 1 節点の手間がどう変わるかを見る。1 スレッドの比べ先は `parprof_20261117`。

- M: kasai・csched008 × 10 設定 × 種 1・2、600 秒 (40 回)、4 分担 (見積り各約 1.2 時間): `python run.py M K` (K = 0〜3)
- P: neos5・noswot・mas76・timtab1・neos-911970・roll3000 × 10 設定 × 種 1・2、120 秒 (120 回)、2 分担 (見積り各約 0.9 時間): `python run.py P K` (K = 0・1)
- **`ENOMOTO_T_MIP_TREE_SYNC_EVERY` を足したコミット以降でビルドして回す** (sync8 の設定が使う)。
- 集計 `python summ.py > summ.txt` (結果が揃ってから書く)。
- `item4.py` / `item4.txt`: 9.75 節の 4 (1 節点の手間の増え方を parprof のログで分ける)。
