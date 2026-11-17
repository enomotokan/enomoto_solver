# 共有を切る・増やす診断 (treeclose 9.75 節の 3、2026-11-17)

4 スレッドで既定に診断の環境変数を 1 つずつ足して (`cfg.py` の 10 設定)、節点の水増しと 1 節点の手間がどう変わるかを見る。1 スレッドの比べ先は `parprof_20261117`。

- M: kasai・csched008 × 10 設定 × 種 1・2、600 秒 (40 回)、4 分担 (見積り各約 1.2 時間): `python run.py M K` (K = 0〜3)
- P: neos5・noswot・mas76・timtab1・neos-911970・roll3000 × 10 設定 × 種 1・2、120 秒 (120 回)、2 分担 (見積り各約 0.9 時間): `python run.py P K` (K = 0・1)
- **`ENOMOTO_T_MIP_TREE_SYNC_EVERY` を足したコミット以降でビルドして回す** (sync8 の設定が使う)。
- 集計 `python summ.py > summ.txt` (結果が揃ってから書く)。
- `item4.py` / `item4.txt`: 9.75 節の 4 (1 節点の手間の増え方を parprof のログで分ける)。

**注意 (2026-11-17)**: 最初の `plan.py` は設定の組を分担ごとに分けてしまった (M の分担 0・2 と P の分担 0 = def・pc1・nobnd・cuts・nofix、
M の分担 1・3 と P の分担 1 = pc0・noproof・sync8・atonce・nobal)。分担は別の機械なので、後の組の def との比には機械の速さの差が混ざる
(節点数は機械によらないので節点の比は使える)。直した `plan.py` は同じ (問題, 種) の全設定を同じ分担に入れる (今の `shards_*.json` は回したときのまま)。
比べ直し: (1) この環境で P の 6 問 × 種 1 × 8 設定を回す (`shards_L.json`、`python run.py L 0`)。(2) M の分担 1・3 と同じ機械で def を回す
(`shards_C.json`、`python run.py C 1`・`python run.py C 3`、各 4 回・600 秒まで)。集計は `python summ.py P`・`python summ.py L` など。
