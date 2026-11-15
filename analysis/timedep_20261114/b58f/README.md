# timedep の 58 問 × 種 1〜4 (6 回目、4 スレッド・60 秒)

設定は `cfg.py`。base = 束 cand1114、D = cand1114 + `ENOMOTO_MIP_TIMEDEP=mplwnNebtyu` + `SEP_COST=5` + `LNS_MIN=10`
(5 回目の組 C から c を外し、u を足したもの)。D-t・D-n・D-u は D から 1 文字ずつ外した形。分担表は `shards.json`
(`python plan.py 8` で作った 8 分担。同じ (問題, 種) の 5 設定は同じ分担。見積り 1 分担 約 1.0 時間)。

## 1 分担の回し方 (4 CPU の機械で、ほかの計測と重ねない)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout fc072e3b   # 解が捨てられる不具合 (af29a20a 以降) の直し 7917d993 が入った版
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py          # 58 問を .miplib_cache に取得
python analysis/timedep_20261114/b58f/run.py K      # K = 0〜7。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `analysis/timedep_20261114/b58f/res_sK.jsonl` だけをコミットして push する。計測中に `git pull --autostash` をしない。
まとめは `python analysis/timedep_20261114/b58f/summ.py > summ.txt`。
