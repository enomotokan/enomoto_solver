# HiGHS 型の木の進め方を束に入れるかの判断 (b58v、analysis/treeclose_20261114.md 9.43 節)

設定は `cfg.py`。土台は timedep で採用した cand1116 + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1`:
- base = 土台
- hgs = 土台 + `ENOMOTO_MIP_DFSBB=2` (HiGHS 型: 潜りは深さ優先で 100 ノードで打ち切り、待ち行列は 10 回に 1 回下界最小、ほかは 0.5 下界 + 0.5 推定値)
- h30 = 土台 + `DFSBB=2`・`ENOMOTO_T_MIP_HGS_BB_EVERY=30`・`ENOMOTO_T_MIP_HGS_PLUNGE_NODES=300`

58 問 × 種 1〜4、4 スレッド・60 秒、9 分担 (`python plan.py 9`)。同じ (問題, 種) の 3 設定は同じ分担に入れる (時期・機械のずれを対で消す)。
1 分担 77 回前後で、見積り約 33 分。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/treeclose_20261114/b58v/run.py K      # K = 0〜8。res_sK.jsonl に足す
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/treeclose_20261114/b58v/summ.py > summ.txt`
(hgs・h30 対 base の SGM と 95% 区間・解けた数・全種で比 > 1.25 / < 0.8・食い違い・片方だけ解けた)、`--ref hgs` で h30 対 hgs。
