# ROBUST の仕上げの対: cand1114 と cand1114 + `ENOMOTO_MIP_ROBUST=pgq` (4 スレッド・60 秒、58 問 × 種 1〜4)

pgq の中身 (記録 `../../robust_20261114.md` の 6 節):
- p・q: 根の目的値のプロービングの見切りを延ばし、候補を 0.5 に近い順に試す。
- g: 暫定解のない根で、安いヒューリスティクスの後ろに FJ・Local-MIP をやり直す。

元の MPS (並べ替えなし) を種 `ENOMOTO_T_MIP_SEED` = 1〜4 で解く。分担表は `shards.json` (`python plan.py 8` で作った 8 分担、
1 分担 58 回で見込み 25 分ほど)。同じ (問題, 種) の 2 組は同じ分担に入れ、交互に解く (機械の違いが試作の効果より大きいため)。

## 1 分担の回し方 (4 CPU の機械で、ほかの計測と重ねない)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>   # この README を足したコミット
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py             # 58 問を .miplib_cache に取得
python analysis/robust_20261114/fin/run.py K          # K = 0〜7。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `analysis/robust_20261114/fin/res_sK.jsonl` と `analysis/robust_20261114/fin/logs/` をコミットして push する。
計測中に `git pull --autostash` をしない。まとめは `python analysis/robust_20261114/fin/summ.py > summ.md`。
