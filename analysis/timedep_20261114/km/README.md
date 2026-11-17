# 手間の単位 (`ENOMOTO_MIP_TICKS=ntwu`) の確かめ (km)

設定は `cfg.py`: base = cand1116f (HALT 入り)、K = base + `ENOMOTO_MIP_TICKS=ntwu`。
Mittelmann 8 問 × 種 1・2、4 スレッド・600 秒、8 分担 (各最長約 40 分)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release

python analysis/timedep_20261114/km/run.py K      # K = 0〜7。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/km/summ.py > summ.txt`。
