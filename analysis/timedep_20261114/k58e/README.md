# 手間の単位 (`ENOMOTO_MIP_TICKS=ntwumcfl`) の確かめ (k58e)

設定は `cfg.py`: base = cand1116f (HALT 入り)、K3 = base + `ENOMOTO_MIP_TICKS=ntwumcfl` (m を直した版)、K3D = K3 + `ENOMOTO_MIP_DETHASH=1`。
58 問 × 種 5〜8 (k58d の種違い)、4 スレッド・60 秒、6 分担 (`python plan.py 6`、各約 0.8 時間。見積りの表示は過去の秒がないので長めに出る)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/k58e/run.py K      # K = 0〜5。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/k58e/summ.py > summ.txt`。
