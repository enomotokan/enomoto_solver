# 手間の単位の r・j の確かめ (k58h)

設定は `cfg.py`:
- base = cand1116 + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` (次の束の形)
- KRJ = base の TICKS を `ntwumcflshkrj` にしたもの (analysis/timedep_20261114.md の 12.5 節。r = 根の LP の同時解きを反復の上限で決める、
  j = FJ・Local-MIP のスレッドの時計を手間にして主と手間の門で受け渡す)

58 問 × 種 1〜4、4 スレッド・60 秒、4 分担 (`python plan.py 4`)。1 分担 116 回で、k58f・k58g の実績からは約 43 分。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/k58h/run.py K      # K = 0〜3。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/k58h/summ.py > summ.txt` (KRJ 対 base)。
