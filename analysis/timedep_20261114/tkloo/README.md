# TICKS の文字ごとの切り分け (tkloo)

cand1117 で neos-827175 (根で解ける) と 10teams が遅い原因の TICKS の文字を探す (analysis/timedep_20261114.md の 12.8 節)。
設定は `cfg.py` の 13 通り:
- cand1117 = cand1116 + TICKS=ntwumcflshk + DETHASH + DFSBB=2
- c17_noticks = cand1117 から TICKS だけを外したもの
- c17_m<文字> = cand1117 の TICKS から 1 文字だけを外したもの (11 通り)

neos-827175・10teams × 種 1〜8、4 スレッド・60 秒、2 分担 (`python plan.py`、各 104 回、約 30 分の見込み)。
neos-827175 は `ENOMOTO_MIP_LOG=1` のログを `logs/<設定>_neos-827175_s<種>.log.gz` に残す。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/tkloo/run.py K      # K = 0・1。res_sK.jsonl と logs/ に足す
```

終わったら `res_sK.jsonl` と `logs/` をコミットして push する。まとめは `python analysis/timedep_20261114/tkloo/summ.py > summ.txt`。
