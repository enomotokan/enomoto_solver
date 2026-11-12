# PRUNE の計測の分担 (手順、2026-11-12)

既定の部品を 1 つずつ止めた形 (leave-one-out、`cfg.py` の 12 設定) と既定 (`def`) を、MIPLIB 58 問 × 種 1・2 × 60 秒・4 スレッドで回す。
同じ (問題, 種) の 13 設定は同じ分担で続けて回す (対で比べる)。ソルバーは 1 つの機械で同時に 1 つ。

## 手順 (1 分担 = 1 セッション、約 1 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>     # 下の「コミット」(既定の動きは HEAD と同じ、台本を足しただけ)
sh analysis/prune_20261112/build.sh                                  # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/prune_20261112
python run.py P <K>                                                  # 途中で切れても同じ命令で続きから回る
```

終わったら `res_P_s<K>.jsonl` だけをコミットして push する (`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後
`git push -u origin ccr-1beed81b-ex0uhq`、失敗したら間を空けて最大 4 回)。途中でも 20 分ごとに push してよい。

- コミット: (下に書く)
- 分担: 0〜11 (`shards_P.json`、見積り各 約 1.04 時間)。10・11 は PRUNE のセッション自身が回す。

| 分担 | 命令 | 結果 |
|---|---|---|
| 0〜9 | `python run.py P 0` 〜 `python run.py P 9` | res_P_s0〜s9.jsonl |
| 10・11 | (PRUNE のセッション) | res_P_s10・s11.jsonl |
