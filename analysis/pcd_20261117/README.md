# 集約の 2 つの試作 pc・pcd の確かめ (2026-11-17)

- def = 既定、pc = `ENOMOTO_AGG_PIVOT_COL=1` (集約のピボットの比を列の側でも判定、analysis/prbsubst_20261117 の 5 節)、
  pcd = pc + `ENOMOTO_AGG_DUALFREE_INEQ=1` (双対で陰に自由な不等式行の等式化、analysis/dualfree_20261117)。`cfg.py`。既定は変えない。
- 動機: sat_rens (satellites2-60-fs の根の RENS の部分問題) が pcd で 40 秒で最適 5 (既定は 120 秒で時間切れ、根の LP −29 → 1.24)。

同じ (問題, 種) の 3 設定を同じ分担で続けて解く (順は種で回す)。4 スレッド。
- P: 58 問 × 種 1〜3、60 秒、4 分担 (見積り各約 1.0 時間): `python run.py P K` (K = 0〜3)
- M: 26 問 × 種 1・2、600 秒、18 分担 (見積り各約 0.92〜0.98 時間): `python run.py M K` (K = 0〜17)
  - cand1119 の M の 22 問 (rootgap の M4 の 12 問・これまでの重い問題・1 ラウンドの LP が重い問題。大きい M の行を持つ数値の難しい問題
    tbfp-network・neos-3216931-puriri・neos-1456979・supportcase33・unitcal_7 などを含む) + sat 系 4 問 (satellites2-40・satellites2-25・
    satellites3-25・satellites4-25)
- 分担表は `python plan.py 4 18` (見積りは cand1119 の回の def の時間 × 3 設定 + 立ち上げ)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy==1.15.1
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/pcd_20261117/run.py P K     # または M K
```

終わったら `res_<P|M>_sK.jsonl` をコミットして push する。

集計 (`summ.py`): `python summ.py P > summ_P.txt`・`python summ.py M > summ_M.txt`・`python summ.py PM > summ_PM.txt`。
pc / def と pcd / def のそれぞれで、採用条件の表 (SGM と 95% CI の上限、解けた数 (P + M の合計が減らないか)、全種で 1.25 倍を超えて遅い問題、
食い違い、crash) を出す。

誤答・数値の害の確かめ (`fuzz.sh`、リポジトリの根から `bash analysis/pcd_20261117/fuzz.sh`): release019 と同じ 7 本 (fuzz_tiny・mip_fuzz_vs_highs・
fuzz_objint・fuzz_slack と種を変えた 3 本) を pcd の環境変数で回し、sat_rens の形の乱数問題 (`analysis/dualfree_20261117/fuzz.py`: 大きい M の行
(係数 1e3〜2e7)・境界が導ける値ちょうどと ±1e-10〜1e-6 の許容の境目) を pcd で 2 万問・pc で 1 万問足す。cargo test は既定のまま。
