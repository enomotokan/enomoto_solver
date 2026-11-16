# 手間の単位 (`ENOMOTO_MIP_TICKS=ntwumcfl`) の確かめ (k58c)

設定は `cfg.py`: base = cand1116f (HALT 入り)、K2 = base + `ENOMOTO_MIP_TICKS=ntwumcfl`。
58 問 × 種 1〜4、4 スレッド・60 秒、4 分担 (`python plan.py 4`、各約 0.8 時間)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/k58c/run.py K      # K = 0〜3。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/k58c/summ.py > summ.txt`。
