# SUBMIP_HALT の確かめ (4 スレッド・60 秒、58 問 × 種 1〜4)

設定は `cfg.py`: base = CAND1115C (TIMEDEP なし)、cH = base + `ENOMOTO_MIP_SUBMIP_HALT=1`、tdH = CAND1115 (TIMEDEP Et2 入り) + `SUBMIP_HALT=1`。
分担表 `shards.json` (`python plan.py 6`、6 分担、同じ (問題, 種) の 3 設定は同じ分担)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/h58/run.py K      # K = 0〜5。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/h58/summ.py > summ.txt`。
