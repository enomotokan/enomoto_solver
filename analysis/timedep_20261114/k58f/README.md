# 手間の単位の s・h・k の確かめ (k58f)

設定は `cfg.py` で、土台は cand1116 (HALT 入り、深さ優先寄りなし):
- base = cand1116
- K3D = base + `ENOMOTO_MIP_TICKS=ntwumcfl` + `ENOMOTO_MIP_DETHASH=1`
- K3DS = base + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` (analysis/timedep_20261114.md の 12.1 節)

58 問 × 種 1〜4、4 スレッド・60 秒、6 分担 (`python plan.py 6`)。k58d・k58e の実績は 1 分担 39〜49 分。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/k58f/run.py K      # K = 0〜5。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/k58f/summ.py > summ.txt`。
