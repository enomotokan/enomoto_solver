# クリークのカットの自動の判定の見直しの確かめ (rootgap の 4 節)

設定は `cfg.py` の 3 通り (どれも環境変数だけで、ソースの既定は変えない):
- c18: CAND1118。クリークのカットは既定の `fallback`: ループの後に 1 回だけ足して、その伸びをループ全体の伸びの半分と比べる。
- c18_clqroot: + `ENOMOTO_MIP_CLIQUE_CUTS=root` (根でいつも回す)
- c18_clqmarg: + `ENOMOTO_T_MIP_CLIQUE_FB_MARGINAL=5` + `ENOMOTO_T_MIP_CLIQUE_FB_SECOND_ROUNDS=10`
  (fallback の判定の相手を、ループの最後の 5 ラウンドの伸びにする。使うときに続ける 2 回目のループは 10 ラウンドまで)

分担 (`python plan.py 4 8`。同じ (問題, 種) の 3 設定は同じ分担で、順は種で回す):
- P: 58 問 × 種 1〜4、60 秒、8 分担 (見積り各約 44 分): `python run.py P K` (K = 0〜7)
- M: mzzv11・mzzv42z・irp・hypothyroid-k1・tbfp-network × 種 1・2、600 秒、4 分担 (見積り各約 40 分): `python run.py M K` (K = 0〜3)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy==1.15.1
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/rootgap_20261116/clq/run.py P K     # または M K
```

終わったら `res_<P|M>_sK.jsonl` (とログがあればそれも) をコミットして push する。集計は `python summ.py P > summ_P.txt`・`python summ.py M > summ_M.txt`。
