# timedep の 58 問 × 種 1〜4 (4 回目、4 スレッド・60 秒、種 1〜4)

設定は `cfg.py` (base = 束 cand1114、ほかは cand1114 + `ENOMOTO_MIP_TIMEDEP` の文字)。分担表は `shards.json`
(`python plan.py 8` で作った 8 分担。同じ (問題, 種) の 5 設定は同じ分担。見積り 1 分担 約 1.1 時間)。

## 1 分担の回し方 (4 CPU の機械で、ほかの計測と重ねない)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout ccr-1beed81b-ex0uhq   # 試作を含む版 (この README を足したコミット以降)
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py          # 58 問を .miplib_cache に取得
python analysis/timedep_20261114/b58d/run.py K      # K = 0〜7。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `analysis/timedep_20261114/b58d/res_sK.jsonl` だけをコミットして push する。計測中に `git pull --autostash` をしない
(書き込み中のファイルが置き換わる)。まとめは `python analysis/timedep_20261114/b58d/summ.py > summ.txt`。
