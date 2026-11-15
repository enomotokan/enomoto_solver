# WINWHY の分担の手順 (2026-11-12)

分担は 1 本 (o600)。このセッションの機械が止まりがちなので、600 秒の組だけを別の機械で回してもらう。

## 手順 (ブランチ ccr-1beed81b-ex0uhq、1 時間ほど)

```
git checkout ccr-1beed81b-ex0uhq && git pull
pip install maturin highspy==1.15.1 && pip install -e .     # リリースのビルド (20〜25 分)
python3 analysis/winwhy_20261111/run.py o600                 # 5 問 x (HiGHS, ENOMOTO 既定・種 1)、600 秒。同時に 1 つ。途中から再開できる
git add analysis/winwhy_20261111/res_o600.jsonl analysis/winwhy_20261111/logs/
git commit -m "WINWHY o600: 完了" && git pull --rebase && git push -u origin ccr-1beed81b-ex0uhq
```

- 問題: mas74・gen-ip054・wachplan・n2seq36q・neos-957323 (`shards.json` の `o600`)。
- 計測の途中で `git pull --rebase` をしない (書きかけのログが消える)。push は全部終わってから。
- 見積り: HiGHS 約 35 分 (3 問は 600 秒の時間切れ)、ENOMOTO 約 15 分。
