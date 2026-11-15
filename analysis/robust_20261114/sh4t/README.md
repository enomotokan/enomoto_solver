# ROBUST の 4 スレッドの比べ: base2 と spgq (58 問 × 6 変種、60 秒)

base2 は束 cand1114 そのもので、spgq は cand1114 に `ENOMOTO_MIP_ROBUST=spgq` を足したもの (`../cfgs.py`)。
変種 (並べ替え, 種) は (0,1) (1,1) (2,1) (3,1) (0,2) (0,3) の 6 つ。並べ替え k ≥ 1 は `../perm.py` が
`.miplib_cache/perm/<問題>_p<k>.mps` に作る (highspy で読み書きし、種 k の乱数で決まる)。

分担表は `shards.json` (`python plan.py 8` で作った 8 分担で、見込みは 1 分担 37 分ほど)。同じ (問題, 変種) の 2 組は同じ分担に入れ、交互に解く。

## 1 分担の回し方 (4 CPU の機械で、ほかの計測と重ねない)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>   # この README を足したコミット (src は 17511b76 と同じ)
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py              # 58 問を .miplib_cache に取得
python analysis/robust_20261114/sh4t/run.py K          # K = 0〜7。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `analysis/robust_20261114/sh4t/res_sK.jsonl` と `analysis/robust_20261114/sh4t/logs/` をコミットして push する
(ログは延長・やり直しが効いた所を見るのに使う)。計測中に `git pull --autostash` をしない。
まとめは `python analysis/robust_20261114/sh4t/merge.py` のあと、`../summ.py res_t4_spgq.jsonl` と `../quick.py 4 spgq base2` で行う。
