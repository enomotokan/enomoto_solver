# TREEEFF の計測の分担 (手順)

どの分担も、同じブランチ `ccr-1beed81b-ex0uhq` の **指定のコミット** をビルドして回す。ソルバーは 1 つの機械で同時に 1 つ (4 スレッド)。

## 手順 (1 分担 = 1 セッション、約 1 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>     # 表の「コミット」
sh analysis/treeeff_20261111/build.sh                                # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/treeeff_20261111
python run.py <SET> <K>                                              # 途中で切れても同じ命令で続きから回る
```

終わったら `res_<SET>_s<K>.jsonl` (と D のときは `logs_D/` の増えたファイル) だけをコミットして push する:

```sh
git checkout -B ccr-1beed81b-ex0uhq origin/ccr-1beed81b-ex0uhq   # 解く側のソースは触らない (コミットを指定して回したので戻す)
git add analysis/treeeff_20261111/res_<SET>_s<K>.jsonl analysis/treeeff_20261111/logs_<SET>/
git commit -m "TREEEFF <SET><K>: 完了 (N 行)"
git pull --rebase --autostash origin ccr-1beed81b-ex0uhq && git push -u origin ccr-1beed81b-ex0uhq   # 失敗したら間を空けて最大 4 回
```

## D: 診断 (600 秒、9 問 × 8 設定 × 種 1・2、144 回、18 分担)

- コミット: **9fde6ff6** (解く側のソースは既定のまま)。
- 設定 (`cfg.py`): `def` 既定 / `trf` 木の中の再スタート (force + GUARD KR) / `nc` ノードの経路集約 (NODECNT=hr) /
  `sd` 退化でも強分岐 / `ca` HiGHS 式のカットの年齢 + 木の LP のカットの行の上限 / `pl` 下界最小の番に共有の待ち行列も見る (TREE1_POOL) / `all` 5 つ全部 / `highs` HiGHS 1.15.1 (random_seed = 種)。
- 見積り: 1 分担 0.2〜1.4 時間 (`shards_D.json` の `est_hours`)。

| 分担 | 命令 |
|---|---|
| D0〜D17 | `python run.py D 0` 〜 `python run.py D 17` (18 分担) |

## P: 58 問の対 (60 秒、58 問 × 種 1〜3 × {def, te, ter}、522 回、5 分担)

- コミット: **(下の行で書き足す)** (`ENOMOTO_MIP_TREEEFF` の試作を足したソース。既定の動きは変わらない)。
- 設定: `def` 既定 / `te` = `ENOMOTO_MIP_TREEEFF=rpsh` / `ter` = `ENOMOTO_MIP_TREEEFF=r` (`cfg.py`)。同じ (問題, 種) は同じ分担で続けて回す。
- 見積り: 1 分担 約 0.9 時間。集計は `python summP.py`。

| 分担 | 命令 |
|---|---|
| P0〜P4 | `python run.py P 0` 〜 `python run.py P 4` |
