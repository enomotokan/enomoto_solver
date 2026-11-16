# SUBMIP_HALT の Mittelmann 版 (4 スレッド・600 秒、8 問 × 種 1・2)

設定は `cfg.py` (base = CAND1115C、cH = base + `ENOMOTO_MIP_SUBMIP_HALT=1`、tdH = CAND1115 + HALT)。8 分担 (1 分担 = 2 組 × 3 設定 = 6 回、最長約 1 時間)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/timedep_20261114/hm/run.py K      # K = 0〜7。res_sK.jsonl に足す (問題は自動で .miplib_cache に取得)
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/hm/summ.py > summ.txt`。
