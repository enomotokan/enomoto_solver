# OBJINT の計測の分担 (手順)

どの分担も、同じブランチ `ccr-1beed81b-ex0uhq` の **コミット 693301c3** をビルドして回す (`ENOMOTO_MIP_OBJINT` の試作入り。既定の動きは変わらない)。
ソルバーは 1 つの機械で同時に 1 つ (4 スレッド)。計測中はチェックアウトを動かさない。

## 手順 (1 分担 = 1 セッション、約 1〜1.5 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout 693301c3
sh analysis/objint_20261112/build.sh                                 # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/objint_20261112
python run.py <SET> <K>                                              # 途中で切れても同じ命令で続きから回る
```

終わったら `res_<SET>_s<K>.jsonl` (T のときは `logs_T/` の増えたファイルも) だけをコミットして push する:

```sh
git checkout -B ccr-1beed81b-ex0uhq origin/ccr-1beed81b-ex0uhq
git add analysis/objint_20261112/res_<SET>_s<K>.jsonl analysis/objint_20261112/logs_<SET>/
git commit -m "OBJINT <SET><K>: 完了 (N 行)"
git pull --rebase --autostash origin ccr-1beed81b-ex0uhq && git push -u origin ccr-1beed81b-ex0uhq   # 失敗したら間を空けて最大 4 回
```

## P: 58 問の対 (60 秒、58 問 × 種 1〜3 × {def, oi}、348 回、3 分担)

- 設定 (`cfg.py`): `def` 既定 / `oi` = `ENOMOTO_MIP_OBJINT=1`。同じ (問題, 種) は同じ分担で続けて回す。見積り 1 分担 約 1.0 時間。

| 分担 | 命令 |
|---|---|
| P0〜P2 | `python run.py P 0` 〜 `python run.py P 2` |

## T: 刻みが新しく付く 47 問側の問題 (600 秒、6 問 × 種 1・2 × {def, oi}、24 回、3 分担、ログつき)

- 問題: csched008・csched007・fiball・neos-3216931-puriri・neos-933966・ns1830653。見積り 1 分担 約 1.4 時間。

| 分担 | 命令 |
|---|---|
| T0〜T2 | `python run.py T 0` 〜 `python run.py T 2` |

## Poic: `c` を足した形の 58 問の対 (60 秒、58 問 × 種 1〜3 × {def, oic}、348 回、3 分担)

- **コミット: 5d36cb0b** (上の手順の `<コミット>` をこれに読み替える。`ENOMOTO_MIP_OBJINT` の `c` 入り)。
- 設定: `def` 既定 / `oic` = `ENOMOTO_MIP_OBJINT=hfc`。見積り 1 分担 約 1.0 時間。

| 分担 | 命令 |
|---|---|
| Poic0〜Poic2 | `python run.py Poic 0` 〜 `python run.py Poic 2` |
