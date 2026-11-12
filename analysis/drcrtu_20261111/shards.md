# DRCRTU の 58 問の対の計測の分担 (analysis/drcrtu_20261111.md)

- 測るもの: 既定 (`def`) と `net` (`ENOMOTO_MIP_FOLD=rinq`、整数の組の TU の判定にネットワーク行列の一般の判定を 2 周目で足す) を、
  58 問 (analysis/implint_tu_20261110/probs_all.py の P58) x 種 1〜3、60 秒・4 スレッドで対にする。同じ (問題, 種) の def と net は
  同じ分担に入る。分担表は `shards_net58.json` (`python plan_net58.py 6` で作った、6 分担・各 58 回・見積り各 0.65 時間)。
- 固定するコミット: このファイルを足したコミット (ブランチ ccr-1beed81b-ex0uhq)。ソースはそのコミットのまま測る。

## 分担 K (0〜5) の手順

```
git fetch origin ccr-1beed81b-ex0uhq && git checkout <固定するコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy pytest
source .venv/bin/activate && maturin develop --release
python analysis/drcrtu_20261111/run_shard.py K net58      # 問題は .miplib_cache/ に自動で取ってくる。途中から再開できる
```

- 出力は `analysis/drcrtu_20261111/net58_sK.jsonl` (58 行)。同時に回すソルバーは 1 つ (4 スレッド)。
- コミットするのは `net58_sK.jsonl` だけ。`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後
  `git push -u origin ccr-1beed81b-ex0uhq` (失敗したら間を空けて最大 4 回)。
