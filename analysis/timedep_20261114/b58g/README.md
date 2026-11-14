# timedep の 58 問 × 種 1〜4 (6 回目、4 スレッド・60 秒)

設定は `cfg.py`。base = 束 cand1114、E = cand1114 + `ENOMOTO_MIP_TIMEDEP=mplwnNebyu` + `LNS_MIN=10` (6 回目の D-t)、
Et2 = E + t (`SEP_COST=2`)、Et5f = E + t (`SEP_COST=5`、`TREE_SEP_TIME_FRAC=0.2`)。分担表は `shards.json`
(`python plan.py 8` で作った 8 分担。同じ (問題, 種) の 4 設定は同じ分担)。

## 1 分担の回し方 (4 CPU の機械で、ほかの計測と重ねない)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>   # この README を足したコミット (src は fc072e3b と同じ)
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py          # 58 問を .miplib_cache に取得
python analysis/timedep_20261114/b58g/run.py K      # K = 0〜7。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `analysis/timedep_20261114/b58g/res_sK.jsonl` だけをコミットして push する。計測中に `git pull --autostash` をしない。
まとめは `python analysis/timedep_20261114/b58g/summ.py > summ.txt`。
