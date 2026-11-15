# 手間の単位 (`ENOMOTO_MIP_TICKS=ntwu`) の確かめ (k58b)

設定は `cfg.py`: base = cand1116f (HALT 入り)、K = base + `ENOMOTO_MIP_TICKS=ntwu`。
58 問 × 種 5〜8 (k58 の種違い)、4 スレッド・60 秒、4 分担 (`python plan.py 4`、各約 0.8 時間)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/k58b/run.py K      # K = 0〜3。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/k58b/summ.py > summ.txt`。
