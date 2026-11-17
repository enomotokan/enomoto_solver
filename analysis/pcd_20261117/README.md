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

## 1. fuzz の結果 (手元、`fuzz_res.txt`、2026-11-17)

- release019 と同じ 7 本 (pcd): すべて bad 0 (fuzz_tiny 400×2・mip_fuzz_vs_highs 300・fuzz_objint 1000×2・fuzz_slack 1000×2)。cargo test (既定) 419 件通過。
- sat_rens の形の乱数問題 (`analysis/dualfree_20261117/fuzz.py`): pcd 2 万問 bad 22、pc 1 万問 bad 15 (試作なしの 1 万問と同じ 15 件)。
  種 10000〜19999 を試作なし・pcd で並べると (どちらも 1 万問):
  - 試作なしだけで食い違う: 12422・15631 (当方が実行不能)、14600・18299 (相対 1e-6 前後の差)。
  - pcd だけで食い違う: 11185 (当方が実行不能)、12914 (当方が 2 だけ悪い。HiGHS の解は列 3 を 3.2e-7 はみ出す。seed 9271 と同じ型の見込み)。
  - 両方: 11480・11988・12020 (HiGHS が実行不能、当方は最適)、17087・17657 (当方が悪い)、18017 (小さな差)、13302・17394 (HiGHS の Solve error)。
- 当方が実行不能とする 3 問 (pcd の 11185、試作なしの 12422・15631) は同じ型 (既定の経路の数値の問題、`analysis/wrongans_20261117`):
  前処理の伝播で連続列の範囲が幅 1e-4〜5e-3 まで締められ (境界が導ける値ちょうど付近の退化に近い問題)、丸めで締めすぎて (HiGHS の最適解が
  相対 4e-10 ほどはみ出す) 実行不能になる。11185 は前処理の後の問題 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED`) が HiGHS でも実行不能、
  12422 も同じ、15631 は伝播で 2 列が 1 点に固まって残りの行が 1.2e-4 破れる。pcd で上げた行 (11185 の 4 本) はどれも最適解で等号
  (元の行 26 を等式にした問題を HiGHS で解いても最適値は同じ)。試作は締めすぎが起きる種を入れ替えるだけ。
- 診断を足した (振る舞いは変えない): `ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED=接頭辞` (前処理の後の問題を MPS に書く)、
  `ENOMOTO_DEBUG_DFI_MAX=k` (上げる行を k 本までにする)。
