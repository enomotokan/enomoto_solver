# TUBR の 58 問の対の計測の分担 (analysis/tubr_20261112.md)

- 測るもの: 既定 (`def`) と `tubr` (`ENOMOTO_MIP_TUBR=1`、分枝しなくてよい整数列 I' を分枝の候補から外す) を、
  58 問 (analysis/implint_tu_20261110/probs_all.py の P58) x 種 1〜3、60 秒・4 スレッドで対にする。同じ (問題, 種) の def と tubr は
  同じ分担に入る。分担表は `shards_tubr58.json` (`python plan_tubr58.py 3` で作った、3 分担・各 116 回・見積り各 1.3 時間)。
- 固定するコミット: このファイルを足したコミット (ブランチ ccr-1beed81b-ex0uhq)。ソースはそのコミットのまま測る。

## 分担 K (0〜2) の手順

```
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq; git checkout <固定するコミット>   # 浅いクローンでは見えないことがある
python -m venv .venv && .venv/bin/pip install maturin highspy pytest
source .venv/bin/activate && maturin develop --release
python analysis/tubr_20261112/run_shard.py K tubr58      # 問題は .miplib_cache/ に自動で取ってくる。途中から再開できる
```

- 出力は `analysis/tubr_20261112/tubr58_sK.jsonl` (116 行)。同時に回すソルバーは 1 つ (4 スレッド)。
- コミットするのは `tubr58_sK.jsonl` だけ。`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後
  `git push -u origin ccr-1beed81b-ex0uhq` (失敗したら間を空けて最大 4 回)。
