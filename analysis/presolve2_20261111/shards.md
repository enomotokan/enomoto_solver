# PRESOLVE2 の計測の分担 (手順)

どの分担も、同じブランチ `ccr-1beed81b-ex0uhq` の **指定のコミット** をビルドして回す。ソルバーは 1 つの機械で同時に 1 つ (4 スレッド)。

## 手順 (1 分担 = 1 セッション、約 1 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>     # 表の「コミット」
sh analysis/presolve2_20261111/build.sh                              # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/presolve2_20261111
python run.py P <K>                                                  # 途中で切れても同じ命令で続きから回る
```

終わったら `res_P_s<K>.jsonl` だけをコミットして push する:

```sh
git checkout -B ccr-1beed81b-ex0uhq origin/ccr-1beed81b-ex0uhq   # 解く側のソースは触らない (コミットを指定して回したので戻す)
git add analysis/presolve2_20261111/res_P_s<K>.jsonl
git commit -m "PRESOLVE2 P<K>: 完了 (N 行)"
git pull --rebase --autostash origin ccr-1beed81b-ex0uhq && git push -u origin ccr-1beed81b-ex0uhq   # 失敗したら間を空けて最大 4 回
```

## P: 58 問の対 (60 秒、58 問 × 種 1〜3 × {def, spl, enum, deq}、696 回、6 分担)

- コミット: **COMMIT** (`ENOMOTO_MIP_DUALEQ` の試作を足したソース。既定の動きは変わらない)。
- 設定 (`cfg.py`): `def` 既定 / `spl` = `ENOMOTO_MIP_SPARSIFY_LATE=1` / `enum` = `ENOMOTO_MIP_HO3_ENUM=1` / `deq` = `ENOMOTO_MIP_DUALEQ=1`。
  同じ (問題, 種) は同じ分担で続けて回す。
- 見積り: 1 分担 約 1.0 時間。集計は `python summP.py`。

| 分担 | 命令 |
|---|---|
| P0〜P5 | `python run.py P 0` 〜 `python run.py P 5` (6 分担) |
