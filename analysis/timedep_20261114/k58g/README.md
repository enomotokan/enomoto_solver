# 手間の単位の s・h・k の確かめ、種 5〜8 (k58g)

設定は `cfg.py` で、土台は cand1116 (HALT 入り、深さ優先寄りなし):
- base = cand1116
- K3DS = base + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` (k58f と同じ)

58 問 × 種 5〜8、4 スレッド・60 秒、4 分担 (`python plan.py 4`)。1 分担 116 回で、k58f の実績からは約 43 分です
(plan.py の見積りは種 5〜8 の過去の時間がないので 1 回 60 秒で数えている)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/k58g/run.py K      # K = 0〜3。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/k58g/summ.py > summ.txt` で、
`--all` で k58f の種 1〜4 と合わせた種 1〜8 になる。
