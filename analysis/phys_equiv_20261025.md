# physiciansched6-2: HiGHS の探り (同値の代入・clique の延長) と ENOMOTO の前処理の形を突き合わせる (依頼 phys-equiv、2026-10-25)

- 機械: 4 CPU (`nproc` = 4)。ソルバーは同時に 1 つだけ (ビルドは計測の合間)。
- 土台: `claude/enomoto-solver-heuristic-survey-xjlnol` の `c1388f8` (既定は mixoc + fix9pe + PRESOLVE_WORK)。作業ブランチ `inv/phys-equiv-20261025`。
- HiGHS: v1.15.1 を源からビルドし、`analysis/highstree_phys_20261024/highs_trace.patch` に計測の出力を足した (`phys_equiv_20261025/highs_trace.patch`、§4)。
- 台本・ログは `analysis/phys_equiv_20261025/` (§5)。前処理の後の問題の書き出し (`dump/`、125 MB) は置いていない (§5 の呼び出しで同じものが出る)。
- 略号: E2 = `ENOMOTO_MIP_PROBE_EQUIV2=1`。POOL = `ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000 ENOMOTO_MIP_CUTSEL_HIGHS=1
  ENOMOTO_MIP_CUTSEL_HIGHS_POOL=1 ENOMOTO_T_MIP_ROOT_POOL_ROUNDS=1000 ENOMOTO_T_MIP_ROOT_CUT_AGE=10`。CCE = `ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1` (今回足した、§3)。
- HiGHS の根のラウンドの値は、計測の値 (前処理後の空間) に node log の最初の暫定解から逆算した定数を足して元の空間に直した (`sumh.py`)。

## 1. 結論

事実:

1. **同値の組: ENOMOTO の E2 の 1084 組は HiGHS の 732 組を全部含む** (`paircmp.py`、符号つきの union-find の同値類で比べた)。
   HiGHS の 732 組はどれも ENOMOTO の類にあって向きも同じ (補 716・同じ 16)。ENOMOTO にしかない組が 337 (533 列)、HiGHS にしかない組は 0。
   類は HiGHS 717 (2 列 702・3 列 15)、ENOMOTO 960 (2 列 898・3 列 62)。HiGHS が見つけない 337 組は、HiGHS の探りで持ち上げ (clique の延長) を止めると HiGHS も見つける
   (持ち上げを止めた回は 872 列を代入、§2.3)。**代入が HiGHS より弱いのではない**。
2. **clique の延長: ENOMOTO は HiGHS より強く延ばしている** (HiGHS: 1744 行に 1812 個、ほとんど 2 文字の行に 1 文字足して 3 文字にする。ENOMOTO: 3644 行に 10857 個、
   支配される行 1451 を消す)。ENOMOTO の延長の後の根の LP は 48583 (HiGHS の前処理の後の LP は 47411) で高いが、その形では根のカットが 49224 で止まる。
3. **根のカットが止まる理由は「根のループで入れるカットが非零 2 の衝突の形のままで、延ばされないこと」**。再スタートの前処理がそのカットの行を延ばすと
   (834 行に 1488 個)、カットを足さないうちに根の LP が 49224 → **49324 (最適値)** になる (E2 + POOL の回、固定 18〜43 本の再スタート)。
4. **手当て CCE (既定では使わない)**: 根のループで LP に入れる集合詰め込みの形のカットを、衝突グラフ (元の行 + 探りの含意) で極大なクリークまで延ばす。
   E2 + POOL + CCE で **根の下界が 4 ラウンド目 (前処理の後 8.2 秒、通しで約 13 秒) に 49324**、RINS が 49324 を見つけて **23.0 秒で最適を証明**
   (診断の回、`logs/physiciansched6-2_e2pool_cce_1.log`)。計測と同じ呼び出しの 4 回は §6。
5. **根の時間の内訳** (E2 + POOL、CCE なし。前処理の後からの時刻): 前処理 4.4 秒 (うち前処理の LP の探りの根の LP 3.3 秒)・対称性 0.8 秒・根の LP 2.3 秒・
   カットの 10 ラウンド 14.8 秒 (1 ラウンドに分離 0.4〜0.6 秒 + LP の解き直し 1.0〜2.4 秒)・根のヒューリスティクス 7.3 秒 → 再スタート (25.9 秒) → 2 回目の根で
   実行可能性ポンプ 17.6 秒。前回の「根に 46 秒」はこの環境では再現せず (カットのループの終わりは前処理の後 17 秒)、**重いのはラウンドの数 (と再スタート後の根)
   で、代入で密になった行による LP の遅さではない** (LP の解き直しは 1 ラウンド 600〜2900 反復)。CCE では 4 ラウンドで 49324 に着き、カットのループは 12.5 秒で
   終わる (49324 に着いた後も 6 ラウンド回っている)。
6. 目的値の不一致 (HiGHS の最適値と違う「最適」) は ENOMOTO の元の問題の回では出ていない。ただし前回の計測版の切り替え `HIGHS_TRACE_NOLIFT` は
   **緩和を書き出していた** (§2.4): 延長しない代わりに、延長した clique に含まれて冗長になった行 (704 行) を消していた。その問題を ENOMOTO にかけると
   「最適 46622」になる (問題が緩和なので ENOMOTO の誤りではない)。§7 (前回) の「持ち上げなし: LP 46619・解なし」は無効。直した版では §2.4。

推測:

- HiGHS の形でカットが速く効くのは、HiGHS の持ち上げが少なく (根の LP 47411)、tableau・集約のカットが LP 解を大きく動かす余地を残すからとみる。
  ENOMOTO は延長で LP を先に 48583 まで上げるが、その面は退化していて、分離器が出すのは主に非零 2 の衝突のカット (`cut rows mean nnz 2`) になる。
  それを延ばせば 1 本で 3〜5 文字のクリークになり、LP がすぐ動く。HiGHS の形に寄せる (延長を弱める) より、カットを延ばす方が手早い。
- 代入 (E2) の効きは、根の LP の大きさ (行 9872 → 8421) と LP の速さ (根の LP 5.1 秒 → 2.3〜2.5 秒) の方にある。E2 なしの POOL + CCE も最適に着くが
  (59.8 秒)、根の LP・ラウンドが倍ほど遅い。

## 2. 比べ (事実)

### 2.1 前処理の後の大きさ・根の LP

| 形 | 行 × 自由な列 (非零) | 根の LP | 備考 |
|---|---|---|---|
| HiGHS の前処理 (元の問題) | 8861 × 4608 (40149) | 47410.8 | 探り 4775 回: 755 列を消す (clique の同値 732)・持ち上げ 1783 |
| ENOMOTO 既定 (延長の前 `def_pre1`) | 12119 × 6003 | 46620 | 探りの固定 180 本 (全部 0)・同値の組 8 |
| ENOMOTO 既定 (延長の後) | (延長 3644 行に 10857 個、支配 1429 行を消す) | 48583.1 | |
| ENOMOTO E2 (延長の前 `e2_pre1`) | 9872 × 4981 (36318) | 46623 | 同値の組 1084 を代入 |
| ENOMOTO E2 (延長の後 `e2_pre2`) | 8421 × 4981 (39802) | 48583.1 | 延長 3644 行に 10857 個、支配 1451 行を消す |
| HiGHS の前処理の後 (`hpre`) に ENOMOTO の前処理 | 7559 × 4608 | 48974 | 延長 402 行に 774 個 |

- HiGHS の列挙 (Enumeration) の固定 157 本のうち 156 本は、ENOMOTO の探りの前の前処理で既に固定されている (`enumfix.py`)。ENOMOTO の探りの固定 180 本は
  HiGHS では探りの中で起きる。列挙は差の元ではない。
- 同値の類の代表に列を写して比べると (`formdiff.py`)、ENOMOTO の形には HiGHS の形にない自由な列が 440 本ある (2 値 303、うち HiGHS の最適解で 1 が 93)。
  HiGHS の前処理の規則を 1 つずつ止めて ENOMOTO の形 (`e2_pre2`) にかけても、列を 300 本消す規則は特定できなかった (`ruleoff.py`、sparsify を止めると 77 本残る程度)。
  根のカットには効いていないとみる (§2.3 で HiGHS の形でなくても 49324 に着く)。

### 2.2 同値の組 (`paircmp.py`、`logs/h_orig.subst`・`dump/e2_0.pairs`)

1. の通り。HiGHS にしかない組は 0、向きの食い違いは 0。

### 2.3 HiGHS を ENOMOTO の形にかける (探りの予算なし `HIGHS_TRACE_PROBEBIG`、60 秒、`sumh.py`)

| 問題 | HiGHS の探り (回、消した列、持ち上げ) | 根のラウンドの値 (元の空間) | 結果 |
|---|---|---|---|
| 元の問題 (`h_orig`) | 4775, 755, 1783 | 48107 → 48678 → 49033 → 49124 → **49324 (5 ラウンド、8.6 秒)** | 12.8 秒で最適 |
| ENOMOTO 既定の延長の前 (`def_pre1`) | 4782, 732, 1745 | 47022 → … → **49324 (11 ラウンド、14.8 秒)** | 16.7 秒 |
| ENOMOTO E2 の延長の前 (`e2_pre1`) | 3911, 0, **517** | 46623 → … → 49236 で止まる | 53.6 秒 (再スタートの後) |
| `def_pre1` + HiGHS の 732 組の等式 | 4136, 124, **580** | 46632 → … → 49000 | 33.6 秒 |
| `def_pre1` + ENOMOTO の 1084 組 | 3909, 0, **140** | 46623 → … → 47523 | 60 秒で 90130 |
| `def_pre1` + ENOMOTO にしかない 337 組 | 4654, 551, 1215 | **49324 (10 ラウンド、12.6 秒)** | 14.6 秒 |
| ENOMOTO 既定の延長の後 (`def_pre2`) | | → 49224 で止まる | 60 秒で 49424 |
| `def_pre2` + 1084 組 | 4242, 0, 0 | **49324 (13 ラウンド、16.9 秒)** | 25.3 秒 |
| `def_pre2` + 732 組 | 4532, 290, 0 | **49324 (12 ラウンド、16.8 秒)** | 22.4 秒 |
| ENOMOTO E2 の延長の後 (`e2_pre2`) | 4242, 0, 0 | → 49057 で止まる | 42.9 秒 |
| `EQUIV_EXTEND_FIRST` の延長の後 (`ef_pre2`、§3) | 4242, 0, 0 | → 49224 で止まる | 29.7 秒 |

- 前回の「ENOMOTO の探りの後では HiGHS の探りの予算が尽きる」は予算を外すと確かめられた (`def_pre1` で 732 組・1745 個に戻り、49324 に着く)。
- **同値を HiGHS の探りの前に代入してしまうと (等式で渡しても、ENOMOTO が代入しても)、HiGHS の持ち上げは 1745 → 140〜580 個に減り、根は 49324 に届かない**。
  HiGHS は探りの中で持ち上げを先に入れてから代入する (`finaliseProbing`)。
- ENOMOTO の延長の後の形では、代入の前後 (def_pre2 + 組、e2_pre2、ef_pre2) の値は 49057〜49324 でばらつく (1 回ずつ、HiGHS は決定的だが入力の小さな違いで
  道が変わる)。ENOMOTO の延長は HiGHS の持ち上げの代わりとして HiGHS のカットに対しては同じくらい効く。

### 2.4 ENOMOTO を HiGHS の形にかける (E2 なし・POOL、60 秒、`run_cfg.sh`)

| 問題 | ENOMOTO の延長 | 根 | 結果 |
|---|---|---|---|
| HiGHS の前処理の後 (`hpre`) | 402 行に 774 個 | 48974 → **49324 (4 ラウンド、8.3 秒)** | **18.3 秒で最適** (RENS で 49324) |
| HiGHS で代入を止めた形 (`hpre_nosubst`、`HIGHS_TRACE_NOSUBST=clique`) | 455 行に 932 個 | 47173 → 49188、再スタートの後 49274 | 47.2 秒で最適 |
| HiGHS で持ち上げを止めた形 (直した `HIGHS_TRACE_NOLIFT`、9751 × 4817、872 列を代入) | 3590 行に 10692 個 | 48624 → 48949、再スタートの後の根の LP 49324 | 時間切れ 49324.5 |
| 同 (直す前の `NOLIFT`、704 行を余分に消した緩和) | | 46620 | 「最適 46622」 (問題が緩和) |

- HiGHS 自身も、直した NOLIFT の形では根が 46623 から動かない (`h_nolift`、60 秒で解なし)。
- **ENOMOTO の延長を通った形 (ENOMOTO の形、HiGHS の NOLIFT の形) では、ENOMOTO の根のループは 48949〜49224 で止まり、再スタートの後に根の LP が 49324 になる**。
  HiGHS の持ち上げを通った形では 4 ラウンドで 49324。

### 2.5 再スタートの後の根の LP が 49324 になる仕組み (`logs/physiciansched6-2_e2pool_keepdom_1.log`)

- 再スタート (固定 18 本、カット 1069 本を行として残す) の前処理: 平行な行の併合 108、探りの同値の組 6、**clique の延長 834 行に 1488 個 (残したカットの行が大半)**。
  その後の根の LP が 49324 (カットのラウンドの前)。
- 根のループで入るカットは毎ラウンド「cut rows mean nnz 2」(非零 2) で、ENOMOTO の延長と同じ規則で延ばせる (`ENOMOTO_T_MIP_CLIQUE_EXTEND_MAX` を 1・2 にすると
  延長の前と同じ LP 46620 になるので、延長は極大まで要る)。`ENOMOTO_MIP_CLIQUE_EXTEND_KEEP_DOMINATED` (支配された行を残す) は根の値を変えない。

## 3. 入れた切り替え (既定では使わない)

| 環境変数 | 場所 | 内容 |
|---|---|---|
| `ENOMOTO_MIP_CUT_CLIQUE_EXTEND` | `src/mip/separation.rs` `extend_clique_cuts`、`root_cut_loop` | 根のカットのループ (サブ MIP 以外) で LP に入れるカットのうち集合詰め込みの形 (0-1 列だけ・係数の絶対値がそろう・文字の和 ≤ 1) を、衝突グラフ (`clique_graph_get`: 元の行 + 探りの含意) と前のラウンドまでに入れた同じ形のカットの文字の組で、極大なクリークまで延ばす (足す文字は LP 解の値の大きい順。1 本に `ENOMOTO_T_MIP_CUT_CLIQUE_EXTEND_MAX` (1000) 個まで)。延ばした後で同じになったカットは 1 本にする。カットの文字の組は `Solver::cut_clique_adj` に覚える (64 文字までのカット) |
| `ENOMOTO_MIP_EQUIV_EXTEND_FIRST` | `src/mip/mod.rs` `substitute_probe_equiv` | 同値の組の等式を足す前に clique の延長をかける (HiGHS の順)。ENOMOTO の延長は代入の後でも同じ量 (3644 行に 10857 個) を足すので、根は変わらない (`ef_pre2`、§2.3)。効かない手として残す |

- `cargo test --release --lib`: 316 件通過・18 件無視。
- CCE は POOL と組むこと: POOL なし (E2 + CCE) では既定の根のループの LP の時間の上限 (1 回 1.2 秒) に 3 ラウンド目で当たり、カットを外して止まる (`e2cce`)。
  CCE だけ (既定のループ) は前景のカットが時間の上限で 0 ラウンド (前回の §1.3 と同じ)。

## 4. HiGHS の計測版に足したもの (`highs_trace.patch`、前回の版からの差分を含む全体)

- `HIGHS_TRACE_SUBSTDUMP=ファイル`: clique の表の同値の代入 (消す列・残す列・向き) を書き足す。
- `HIGHS_TRACE_LIFTDUMP=ファイル`: 持ち上げ (行の名前・列の名前・向き) を書き足す。
- `HIGHS_TRACE_ENUMDUMP=ファイル`: 列挙で固定・締めた列と、列挙した行の数・見つけた clique の数。
- `HIGHS_TRACE_PROBEBIG`: 外側の問題の探りを予算 (`probingContingent`・`splayContingent`) で打ち切らない。
- `HIGHS_TRACE_NOLIFT` の直し: 持ち上げを入れないときは、延長した clique に含まれて冗長になった行 (`getDeletedRows`) も消さない (直す前は緩和を作っていた)。

## 5. 台本・ログ (`analysis/phys_equiv_20261025/`)

- `run_highs.sh <名前> <秒> [mps]`: 計測版 HiGHS (`HIGHS_BIN`、既定 `/home/user/HiGHS/build/bin/highs`) で `logs/<名前>.{log,tr,subst,lift}`。
- `run_cfg.sh <問題 or mps> <秒> <名前> [ENV=VAL ...]` (前回から写し): ENOMOTO を `ENOMOTO_MIP_LOG=1` で 1 回、`logs/<問題>_<名前>_<n>.log`。`eno_run.py` も写し。
- 前処理の書き出し: `ENOMOTO_MIP_DEBUG_DUMP_PROBE=dump/<c> ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED=dump/<c>_pre1.mps ..._PRESOLVED2=dump/<c>_pre2.mps
  ..._PRESOLVED3=... ENOMOTO_MIP_DEBUG_STOP_PRESOLVED=1` (`logs/eno_pre_{def,e2,ef}.log`)。HiGHS の前処理の後は highspy の `writePresolvedModel` (`hpre`) と
  `write_presolved_model_file` (`hpre_{full,nolift,nosubst}`、`logs/wp_*.log`)。
- `paircmp.py` (同値の組)・`addpairs.py` (組の等式を足す)・`formdiff.py` (形の突き合わせ)・`enumfix.py` (列挙の固定)・`liftex.py` (持ち上げの例)・`ruleoff.py`
  (HiGHS の規則を 1 つずつ止める)・`sumh.py` (HiGHS のログの要約)・`sumeno.py` (ENOMOTO のログの要約)。
- `batch_pe.sh`: §6 の計測。結果は `benchmarks/{highsonly16,miplib_results}_4t_60s_20261020_20bc1ec_{pecce,pedef}_r*.json`、`benchmarks/cand_logs/{pecce,pedef}_r*.log`。

## 6. 確かめ (計測と同じ呼び出し `bench_mix.sh`、60 秒・4 スレッド、`ENOMOTO_BENCH_SHA=20bc1ec`)

候補 pecce = E2 + POOL + CCE (環境変数そのまま: `ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000
ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_MIP_CUTSEL_HIGHS_POOL=1 ENOMOTO_T_MIP_ROOT_POOL_ROUNDS=1000 ENOMOTO_T_MIP_ROOT_CUT_AGE=10 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1`)。

RESULTS_PLACEHOLDER
