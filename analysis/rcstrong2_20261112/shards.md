# RCSTRONG2 の計測の分担 (手順): 58 問の対 irk と k

58 問 × 種 1〜3 × {irk, k}、60 秒、348 回、3 分担 (1 分担 約 1.0 時間)。同じ (問題, 種) の irk と k は同じ分担で続けて回す
(表は analysis/clqprb_20261112/shards_P.json の def → irk、clq → k の置き換え)。

- 設定 (`cfg.py`): `irk` = `ENOMOTO_MIP_RCSTRONG=irk` (束 cand1113 の形、対の基準) / `k` = `ENOMOTO_MIP_RCSTRONG=k` (暗黙の整数の根の固定 `i` を外した形)。
- コミット: このファイルを足したコミット (親への報告に書く)。ソースは 9417ecb2 から変えていない (試作 RCSTRONG はもとから入っている)。
- ソルバーは 1 つの機械で同時に 1 つ (4 スレッド)。計測中はチェックアウトを動かさない。

## 手順 (1 分担 = 1 セッション)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>
sh analysis/rcstrong2_20261112/build.sh                  # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/rcstrong2_20261112
python run58.py P <K>                                     # K = 0〜2。途中で切れても同じ命令で続きから回る
```

終わったら `res_P_s<K>.jsonl` だけをコミットして push する:

```sh
git checkout -B ccr-1beed81b-ex0uhq origin/ccr-1beed81b-ex0uhq
git add analysis/rcstrong2_20261112/res_P_s<K>.jsonl
git commit -m "RCSTRONG2 P<K>: 完了 (N 行)"
git pull --rebase --autostash origin ccr-1beed81b-ex0uhq && git push -u origin ccr-1beed81b-ex0uhq   # 失敗したら間を空けて最大 4 回
```

| 分担 | 命令 | 結果 |
|---|---|---|
| P0〜P2 | `python run58.py P 0` 〜 `python run58.py P 2` | `res_P_s0.jsonl` 〜 `res_P_s2.jsonl` |

集計: `cd analysis/rcstrong2_20261112 && python summP.py` (基準 irk、候補 k)。
