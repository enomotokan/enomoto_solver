# CLQPRB の計測の分担 (手順)

どの分担も、同じブランチ `ccr-1beed81b-ex0uhq` の **指定のコミット** をビルドして回す。ソルバーは 1 つの機械で同時に 1 つ (4 スレッド)。
計測中はチェックアウトを動かさない。

## 手順 (1 分担 = 1 セッション、約 1 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>     # 表の「コミット」
sh analysis/clqprb_20261112/build.sh                                 # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/clqprb_20261112
python run.py P <K>                                                  # 途中で切れても同じ命令で続きから回る
```

終わったら `res_P_s<K>.jsonl` だけをコミットして push する:

```sh
git checkout -B ccr-1beed81b-ex0uhq origin/ccr-1beed81b-ex0uhq   # 解く側のソースは触らない (コミットを指定して回したので戻す)
git add analysis/clqprb_20261112/res_P_s<K>.jsonl
git commit -m "CLQPRB P<K>: 完了 (N 行)"
git pull --rebase --autostash origin ccr-1beed81b-ex0uhq && git push -u origin ccr-1beed81b-ex0uhq   # 失敗したら間を空けて最大 4 回
```

## P: 58 問の対 (60 秒、58 問 × 種 1〜3 × {def, clq}、348 回、3 分担)

- コミット: **7f2b082c** (試作 `ENOMOTO_MIP_CLQPRB` を足したソース。既定の動きは変わらない)。
- 設定 (`cfg.py`): `def` 既定 / `clq` = `ENOMOTO_MIP_CLQPRB=1`。同じ (問題, 種) は同じ分担で続けて回す。
- 見積り: 1 分担 約 1.0 時間。集計は `python summP.py`。

| 分担 | 命令 |
|---|---|
| P0〜P2 | `python run.py P 0` 〜 `python run.py P 2` (3 分担) |

## C: Mittelmann 186 問の整数列の幅・暗黙の整数の数え上げ (診断だけ、6 分担)

- コミット: **73d9bf43** (診断 `ENOMOTO_MIP_DEBUG_INTCENSUS` を足したソース。既定の動きは変わらない)。手順のビルドまでは上と同じ。
- 命令: `python census.py mitt186.txt <K> 6` (K = 0〜5)。問題ごとに根の後の行が出るか 150 秒で止める (最大 約 3.5 分 × 31 問 ≒ 1〜1.5 時間)。
- 結果: `census_mitt186_def_s<K>.jsonl` だけをコミットして push (コミットの文は `CLQPRB C<K>: 完了 (N 行)`)。

| 分担 | 命令 |
|---|---|
| C0〜C5 | `python census.py mitt186.txt 0 6` 〜 `python census.py mitt186.txt 5 6` |

## A: Mittelmann 186 問の CLQACT の数え上げ (診断だけ、3 分担)

- コミット: **778566ed** (診断 `ENOMOTO_MIP_DEBUG_CLQACT` を足したソース。既定の動きは変わらない)。手順のビルドまでは上と同じ。
- 命令: `python census.py mitt186.txt <K> 3 act` (K = 0〜2)。`CLQPRB=1` で前処理の後の CLQACT の行が出たら止める (最大 約 3.5 分 × 62 問)。
- 結果: `census_mitt186_act_s<K>.jsonl` だけをコミットして push (コミットの文は `CLQPRB A<K>: 完了 (N 行)`)。

| 分担 | 命令 |
|---|---|
| A0〜A2 | `python census.py mitt186.txt 0 3 act` 〜 `python census.py mitt186.txt 2 3 act` |

## Q: 58 問の対 clq と clqa (60 秒、58 問 × 種 1〜3 × {clq, clqa}、348 回、3 分担)

- コミット: **e9041ef0** (試作 `ENOMOTO_MIP_CLQACT` を足したソース。既定の動きは変わらない)。手順は P と同じで、命令が `python run.py Q <K>`、結果が `res_Q_s<K>.jsonl`。
- 設定 (`cfg.py`): `clq` = `ENOMOTO_MIP_CLQPRB=1` (対の基準) / `clqa` = `ENOMOTO_MIP_CLQPRB=1 ENOMOTO_MIP_CLQACT=1`。
- 見積り: 1 分担 約 1.0 時間。集計は `SET=Q BASE=clq python summP.py`。

| 分担 | 命令 |
|---|---|
| Q0〜Q2 | `python run.py Q 0` 〜 `python run.py Q 2` (3 分担) |
