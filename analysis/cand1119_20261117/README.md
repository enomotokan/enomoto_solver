# 束 cand1119 の確かめ (2026-11-17)

cand1119 = 既定 (cand1118) + 次の環境変数 (`cfg.py`。既定は変えない):
- `ENOMOTO_MIP_CLIQUE_CUTS=sigmoid`・`ENOMOTO_T_MIP_CLIQUE_SIG_A=1.8`・`ENOMOTO_T_MIP_CLIQUE_SIG_B=0.41` (analysis/rootgap_20261116/clq の 2〜5 節)
- `ENOMOTO_MIP_SYM_TWINS=1` (同じ README の 6 節)
- `ENOMOTO_MIP_CUTSEL_PROBE_AFFORD=1` (analysis/cutprobe_20261117 の aff)

def (環境変数なし) と cand1119 を、同じ (問題, 種) で同じ分担に続けて解く (順は種で入れ替える)。4 スレッド。
- P: 58 問 × 種 1〜6、60 秒、6 分担 (見積り各約 57 分): `python run.py P K` (K = 0〜5)
- M: 22 問 × 種 1〜3、600 秒、14 分担 (見積り各約 56 分): `python run.py M K` (K = 0〜13)
  - rootgap の M4 の 12 問: mzzv11・mzzv42z・irp・hypothyroid-k1・tbfp-network・neos-933966・mcsched・csched008・neos-1456979・
    neos-3216931-puriri・traininstance6・satellites2-60-fs
  - これまでの M の重い問題: neos-3627168-kasai・rococoC10-001000・supportcase33・unitcal_7・net12
  - 1 ラウンドの LP が重い大きめの問題 (cand1118 か HiGHS 1.15.1 が 600 秒で解いたもの): neos-4722843-widden・CMS750_4・
    neos-4738912-atrato・fastxgemm-n2r6s0t2・istanbul-no-cutoff
- 分担表は `python plan.py 6 14` (見積りは cand1118 の回の時間 + 立ち上げ)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy==1.15.1
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/cand1119_20261117/run.py P K     # または M K
```

終わったら `res_<P|M>_sK.jsonl` をコミットして push する。

集計 (`summ.py`): `python summ.py P > summ_P.txt`・`python summ.py M > summ_M.txt`・`python summ.py PM > summ_PM.txt`。
採用条件の表 (SGM cand1119 / def と 95% CI の上限、解けた数 (P + M の合計が減らないか)、全種で 1.25 倍を超えて遅い問題、食い違い、crash) を出す。

誤答の確かめ (`fuzz.sh`): analysis/cand1118_default_20261116/fuzz.sh と同じ組 (fuzz_tiny・mip_fuzz_vs_highs・fuzz_objint・fuzz_slack と
種を変えた 3 本) を cand1119 の環境変数を付けて回し、双子の併合が働く同じ整数列の乱数問題 (analysis/parint_20261028/fuzz_dupcols.py 1000 問) を足す。
cargo test は既定のまま。リポジトリの根から `bash analysis/cand1119_20261117/fuzz.sh`。手元で fuzz_dupcols 100 問を cand1119 の環境変数で回し、不一致 0。

## 2. 部品の切り分け (A の組)

M で csched008 が全種で 1.25 倍を超えて遅い (1.30・1.80、種 3 は両方時間切れ)。ほかに neos-933966 1.83 (種 1・3 は cand1119 だけ時間切れ)・CMS750_4 1.25・
supportcase33 1.22。どの部品が元かを見るため、cand1119 から部品を 1 つずつ抜いた設定を同じ分担で並べる (`cfg.py`):
def・cand1119・no_sig (シグモイドの 3 つを抜く)・no_twins (SYM_TWINS を抜く)・no_aff (CUTSEL_PROBE_AFFORD を抜く)。
- A: csched008・neos-933966・CMS750_4・supportcase33 × 種 1〜3 × 5 設定、600 秒、6 分担 (見積り各 62〜68 分、`python plan.py 6 A`): `python run.py A K` (K = 0〜5)
- 集計: `python summ.py A > summ_A.txt` (問題・種ごとの秒の表と、問題ごとの「設定 / def」の幾何平均)
