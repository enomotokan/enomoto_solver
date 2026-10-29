# CBC (COIN-OR) と ENOMOTO の比較: ENOMOTO にない高速化・無駄の削減の候補 (2026-10-29)

## 調査の条件

- **ENOMOTO**: develop 7848a512 (version 0.1.5)。
- **COIN-OR**: 各リポジトリの master を `--depth 1` で取得 (2026-10-29)。
  - Cbc a8e33a5 (2026-10-28)
  - Cgl 2c48cb7
  - Clp d4baf22
  - CoinUtils 0073dcd
  - Osi f9502c1
- **進め方**: ソースを読んで比べただけで、ビルドと計測はしていない。
  - 期待効果は、コードと既存の記録 (主に `analysis/sepspeed_20261023.md`、`analysis/lp_hang_20261021.md`、`analysis/mip_survey_20261011.md`) から見込んだもの。
  - 各候補について、ENOMOTO に既にあるか、試して不採用になっていないかを、ソース・`ENOMOTO_*` の環境変数・`docs/improvement_history.md`・`analysis/*.md` で確かめた。
- **領域**: LP 単体法、木の探索、切除平面、前処理の 4 つに分けて調べた。
- **行番号**: ENOMOTO・COIN とも上の版のもの。

## 全体のまとめ

**LP エンジンの本体**には、Clp の方が速い所は見つからなかった。CHUZR、BFRT/Harris、DSE、超疎の FTRAN/BTRAN、FT 更新、密な LU、行コピーの PRICE は、HiGHS 流のものが既にある。差が残るのは「Cbc が Clp をどう呼ぶか」の部分で、次の 3 つ。

- ノード LP での摂動
- 目的関数だけを変える再求解での主単体法
- 求解ごとの準備の固定費

**MIP の側**では、CBC にあって ENOMOTO にない仕組みが主に 3 系統ある。

- 分離器ごとの適応的な間引き
- 目的関数の刻みの広い判定
- プロービングによる行の係数の強化

**比べる途中で見つかった ENOMOTO だけの無駄**もある。

- 根のカットのループで、毎ラウンド LU を捨てている
- 元の行の分離が、経路集約と重なっている

### 優先順位の案 (効果 ÷ 手間の大きい順)

| # | 候補 | 領域 | 期待効果 | 難しさ | 経路 | 前例 |
|---|---|---|---|---|---|---|
| 1 | 根のカットのループで、効いていないカットを外す時期を、カットを足す直前に移す (LU を捨てない) | カット | 1 ラウンドあたり分解 1 回と標準形の作り直し 1 回が減る | 小 | ほぼ不変 | C10/W5 に挙がっただけ |
| 2 | 分離器ごとの適応的な間引き (CBC の adaptive skip と、根の後の `howOften` の決定) | カット | 根の分離の時間 −30〜60% (roll3000・physiciansched6-2) | 小〜中 | 変わる | なし |
| 3 | 目的関数の刻み `obj_step` を、暗黙の整数列まで広げる (`analyzeObjective`) | 木 | 該当する問題で刈り込みが「相対 1e-9」から「刻み 1 つ」になる | 小 | 変わる | `OBJ_STEP_SKIP_FIXED` だけ |
| 4 | 主単体法の CHUZC の全列走査をやめる (実行不能リスト) | LP | 列の多い LP の polish・篩い分け法 | 低〜中 | ビット一致にできる | なし |
| 5 | 強分岐の子の LP 値から、凸性で一般整数の境界を締める (`CBCNODE_TIGHTEN_BOUNDS`) | 木 | 追加の LP なしにノードの境界が締まる | 小 | 変わる | なし |
| 6 | プロービングで一般の行の係数を強める (CglProbing の `strengthenRow`) | 前処理 | 根の LP の値が上がる (big-M・容量の行) | 中 | 変わる | なし |
| 7 | 元の行の分離と経路集約の重複をなくし、ノードでは締まった行だけを使う | カット | 元の行の分離 −50% 程度 | 小 | 変わる | FRACSKIP (別の判定) は効果なし |
| 8 | 元の行・zerohalf・clique・implied bound の分離を並列にする (`parallelCuts`) | カット | 根の 1 ラウンドの壁時間 −10〜20% | 中 | 不変 | tableau と従来の経路集約だけ並列 |
| 9 | zerohalf の前処理 (gcd で割る、偶数だけの行を消す、同じパリティの行をまとめる、列から行への索引) | カット | zerohalf の時間 −30〜70% | 小〜中 | ほぼ不変 | なし |
| 10 | ノード LP で、費用の値の種類が少ない問題に限って摂動する | LP | 双対退化の強い問題でノード LP の反復が減る | 中 | 変わる | 試されていない |
| 11 | 含意の境界を単体法に渡さない (ゆるめて渡す) (`tightenPrimalBounds`) | 前処理 | irish で反復 −10〜20% と見積もりあり | 中 | 変わる | 「範囲が広いので見送り」 |
| 12 | 目的関数だけを変える再求解を主単体法で解く (FPump の `OsiDoDualInResolve=false`) | LP | FP など費用を変える再求解の反復 | 中 | 変わる | なし (4 を先にする) |

小さな無駄の削減で、経路をほぼ変えないものもある。探索を変えない高速化の束として、まとめて測るのに向く。

- カットの LP が失敗したときにカットの前の基底へ戻す (カット 9)
- `separate()` で LP の行を毎回複製しない (カット 10)
- 毎ノードの `zi_round` の O(nnz) (木 5a)
- `presolve_mip_depth` の詰め替え (前処理 8)
- 求解ごとの準備の O(n_total) (LP 4)

---

## 1. LP 単体法

### LP1. ノード LP の費用の摂動 (効果: 中〜大、難しさ: 中)

**Clp/Cbc 側**
- `ClpSimplexDual.cpp:336–365` (`startupSolve`) は、Cbc の中 (`inCbcOrOther`) で始めに主実行不能なら `perturb()` を呼ぶ。
- `fastDual` (7486–7491) は、反復が 2(m+n) を超えて進まないときにも摂動する。
- `perturb()` (6591–) は、構造列の |費用| の相異なる値を数え、`number*4 > numberColumns` なら摂動しない (6650–6662)。費用の値の種類が少なく双対の同点が多い問題だけを摂動する。

**ENOMOTO 側**
- `slope_intercept_dual.rs:3561–3563` は、`FAST_REOPT` (MIP の warm start、既定) では `active_cost = std.c.clone()` で、摂動しない。
- 退化への対処は `DEGEN_SHIFT_RUN` (厳密な退化ピボットが 10 回続いた後) と Bland だけ。

**根拠**
- `analysis/lp_hang_20261021.md`: warm start では `ENOMOTO_T_PERTURB_FACTOR` を変えても反復は変わらない (摂動が入らないため)。
- `analysis/neos2978193_r3_20261020.md`: ノード LP の反復は HiGHS の約 2 倍。

**注意**: 摂動すると `fast_finish` が使えず、仕上げの費用が増える。仕上げの増分を含めて測る。

### LP2. 目的関数だけを変える再求解を主単体法で (効果: 中、難しさ: 中)

**Clp/Cbc 側**: `CbcHeuristicFPump.cpp:847`/2145 は `setHintParam(OsiDoDualInResolve, false)` とする。`OsiClpSolverInterface.cpp:1248`→1554 で `primal(1)` を呼ぶ。

**ENOMOTO 側**
- `mip_lp.rs:723–725` の `solve_primal` の中身は、双対単体法の `self.solve(lim)`。
- 使う場所は FP (`heuristics.rs:870–873`)、`heuristics.rs:2549/2566`、`separation.rs:2892`、`solver.rs:2882/2920`。
- `warm_start_basis` は双対実行不能な箱型の列を反対の境界へ置き直すので、主実行可能性が壊れる。その後、双対単体法と polish の 2 段で解いている。

**案**: 基底が主実行可能なら、真の費用で `run_phase2_incremental` を直接回す。LP3 を先に入れる。

### LP3. 主単体法の CHUZC の全列走査 (効果: 列の多い問題で中、難しさ: 低〜中)

**Clp 側**: `ClpPrimalColumnSteepest.cpp:215` (`pivotColumn`) は、双対実行不能な列を `infeasible_` に持ち、触った列だけを更新する (`quickAdd`、417–439)。大きな問題では `partialPricing` (4188–) に切り替える。

**ENOMOTO 側**
- `simplex.rs:1411–1437` (`run_phase2_incremental`) は毎反復 `for j in 0..n` で全列を比べる (確認済み)。
- d の更新は触った列だけなのに、選択は O(n_total)。
- 篩い分け法 (`sifting.rs:372`) もこの関数を使う。
- lp_hang の LP (111827 列) では、主単体法への引き継ぎが 20〜3210 回のピボットあった。

**案**: 双対実行不能な列の集合を差分で持つ。選び方を同じにすれば、経路はビット一致にできる。

### LP4. 求解ごとの準備の O(n_total) (効果: 反復の少ない LP と強分岐で中、難しさ: 低〜中)

**Clp 側**: `ClpSimplexDual::strongBranching` (7021–) は、全候補で `createRim` と分解を 1 回だけ行い、memcpy と `setFactorization` で戻す。`startFastDual2`/`fastDual2` (`ClpSimplex.cpp:13207/13290`) は作業配列を残す。

**ENOMOTO 側**: `solve_slope_intercept_dual_impl` の warm start 1 回あたり、次の処理がある。
- O(n_total) の確保・走査が約 20 回 (3568 `delta`、3572、3597、3603、3648 の 2 つ目の `ColCache::build`、3668 `d.clone`、3776 `repartition` など)。
- O(nnz) の処理が 3〜4 回。`warm_start_basis` の d の計算では、固定列も含めて内積を取る。

強分岐の反復上限は 10〜500 なので、準備の割合が大きい。強分岐の LP は ENOMOTO 約 1800 反復/秒、HiGHS 約 2700 反復/秒 (`mzzcmp_20261023.md:123`)。

**案**
1. まず `ENOMOTO_MIP_XPROF` で内訳を測る。
2. 固定列を d のループと `ColCache` で飛ばす。
3. 作業ベクトルを使い回す (L6)。

**前例**: L6 は評価の記録がない。W・WC は単独では不採用で、WLC の束だけ採用。

### LP5. 縮めたノード LP (crunch) (効果: 固定の多い LP で中、難しさ: 高)

**Clp/Cbc 側**: `ClpSimplexOther::crunch` (`ClpSimplexOther.cpp:1781`) は、固定列と、列 1 本だけの行などを落とした小さい LP を解く。`KEEP_SMALL` は、境界が内側なら作り直さない (`OsiClpSolverInterface.cpp:7984–8020`)。

**ENOMOTO 側**: 固定列は標準形に残し、PRICE から外すだけ (`PriceMat::priced`)。lp_hang では 111827 列のうち 106846 列が固定だった。

**評価**: 番号の写像 (カット・双対・基底・DSE 重み) が重い。LP4 の「固定列を飛ばす」で取れる分を先に取る。

### LP6. fathom (効果: 低〜中、難しさ: 高)

**Cbc 側**: `fastNodeDepth_` を設定すると、深いノードを `ClpSimplex::fathom` (`ClpSimplex.cpp:11731`) に渡し、Clp の中だけでカットなしの深さ優先で解く。

木の側の仕組みなので、木 9 (fathomMany) と合わせて扱う。

### LP7. 更新回数に応じた許容ピボット (効果: 低、安定性)

**Clp 側**: `ClpSimplexDual.cpp:1270–1278`。

**ENOMOTO 側**: `PIVOT_ESCALATE_FEW_UPDATES` (既定 0) が近い。数値の不具合が出たときに試す手として記録だけしておく。

### LP: 既にある・不採用のもの

- **既にある**
  - DSE の CHUZR: 実行不能リスト + 遅延ヒープで、Clp より厳密。
  - DSE 重みの引き継ぎ (`saveWeights` に当たる)、厳密な重みの初期化。
  - BFRT/Harris、行コピーの PRICE、tau の FTRAN の融合、超疎の切り替え、密な LU、Markowitz の探索の打ち切り。
  - `LuCache` と兄弟ノードの `lu_snapshot`。
- **試して不採用**: 双対目的値での毎反復の打ち切り (L2、+1.59%)、強分岐の途中の下界 (L1、+1.52%)。
- **Clp でも既定で使わないので対象外**: `CLEAN_FIXED` (`#define 0`)、`goDense`/`goSmall` (既定 -1)、行の追加での分解の拡張。
- **確かめきれなかったこと**: この版の Cbc の `MIPOPTIONS` の既定値 (`CbcParameters.cpp:1126` は 0、`CbcSolver.cpp:8821` は 1057/1025 を普段の値として扱う)。そのため crunch が既定で効くかは確かめていない。

---

## 2. 木の探索・ノード処理

### 木1. 目的関数の刻みを暗黙の整数列まで広げる (難しさ: 小、未試行)

**CBC 側**: `CbcModel::analyzeObjective` (CbcModel.cpp:438–) は、費用を持つ連続列があっても、右辺が整数でネットワーク・被覆の形 (`continuousMultiplier`) なら整数とみなし、`CbcCutoffIncrement` を付ける。

**ENOMOTO 側**
- `problem.rs:374–407` の `objective_step_bounds` は、費用のある連続列が 1 本でもあれば `None` を返す (確認済み)。
- 暗黙の整数の判定 `implied_integers()` (problem.rs:215–) は既にあるが、`cut_int` (solver.rs:2051–2062) としてカット生成でしか使っていない。

**正しさの注意**
- 双対側の暗黙の整数は「ある最適解では整数」という意味しか持たない。
- 暫定解の目的値が刻みの格子の上にあるときだけ、刻みで刈る。
- `orig_only` や時間切れの経路は `resolve_continuous` を通らないので、格子に乗らない。
- 他スレッドが共有する最良値も、同じ判定にする。

### 木2. 強分岐の子の LP 値で一般整数の境界を締める (難しさ: 小、未試行)

**CBC 側**: CbcNode.cpp:3853–3881 (下の子)、4151–4181 (上の子) の `CBCNODE_TIGHTEN_BOUNDS`。
- `changePer = Δ/(v − floor v)`、`distance = (cutoff − z)/changePer + 1e-3` を計算する。
- `distance < 5` なら `x_j >= ceil(v − distance)` などを置く。
- LP の値は境界の関数として凸なので、これは正しい。

**ENOMOTO 側**: solver.rs:7189–7205 (並列の経路は 7108–7130) は、利得を記録するだけ。

### 木3. 強分岐の下の子にも orbital fixing を当てる (難しさ: 小〜中、未試行)

**CBC 側**: CbcNode.cpp:3660–3690 は、下の子を解く前に軌道の列を 0 にする。

**ENOMOTO 側**: 強分岐の下の子 (solver.rs:7182–7187) には `x_j <= floor` だけを置く。本当の子では `orbital_fixing` (7739–7847) が軌道を全部 0 にするので、強分岐の評価が本当の子より弱く見える。

**注意**: 双対証明 (`sb_child_proof`) も、置いた固定を扱うよう広げる必要がある。

### 木4. 強分岐で片側を固定したら、すぐ LP を解き直して続ける (難しさ: 中、未試行)

**CBC 側**: CbcNode.cpp:4440–4480 は、固定したら `resolve` して `markHotStart` し直す。HiGHS も同じ。

**ENOMOTO 側**: 固定は定義域にだけ置き、残りの候補は古い LP で強分岐する (solver.rs:7231–7259)。5 個たまると `Resolve` を返して `select_branch` 全体をやり直す (5787–5797)。

**前例**: B2m (ノード内の記憶、+2.97% で不採用) とは別物。

### 木5. 毎ノードの ZI round (難しさ: 小)

**CBC 側**: `CbcHeuristic::shouldHeurRun_randomChoice` (CbcHeuristic.cpp:460–) は、深さ d で確率 d²/2^d で回し、失敗が続くと間隔を広げる。

**ENOMOTO 側**: solver.rs:5292–5297 が毎ノード `simple_rounding` と `zi_round` を回す。`zi_round` (heuristics.rs:2849–2855) は、毎回全行の活動量を作り直す。

**案**
- (a) 経路を変えない: 活動量を `row_activities_ref()` から取る。
- (b) 成功率で間引く。

**前例**: W7 に挙がったが、WLC には入っていない。

### 木6. 打ち切り・実行不能になった子も、擬似費用の信頼度に数える (難しさ: 小、未試行)

**CBC 側**: CbcSimpleIntegerDynamicPseudoCost.cpp:1205–1242 (`INFEAS == 1`)。

**ENOMOTO 側**: `add_cutoff` (pseudocost.rs:167–173) は回数を数えるだけで、信頼度 `min_observations` (212–214) は `n_up`/`n_down` しか見ない。HiGHS も数えない。

### 木7. ダイビングでの被約費用による固定と、整数値の列の固定 (難しさ: 小〜中)

**CBC 側**: CbcHeuristicDive.cpp:684 (`reducedCostFix`)、767 (`fixOtherVariables`)。整数値の整数列を |被約費用| の大きい順に、整数列の 20% まで固定する。

**ENOMOTO 側**: `dive` (heuristics.rs:510–744) は 1 ステップで 1 列だけ動かす。

**前例**: 分数の列をまとめて固定する P6 は不採用。CBC の形 (整数値の列、LP の最適性を崩さない) は未試行。

### 木8. 待ち行列のノードの基底を差分・2 ビットで持つ (主にメモリ、難しさ: 中)

**CBC 側**: `CbcPartialNodeInfo` は親との差分だけを持つ。

**ENOMOTO 側**: solver.rs:5864 で、子 2 つにつき n+m バイトを持つ。n+m = 10 万、10 万組で約 10 GB になる。

### 木9. 小さな問題の部分木を軽いノード処理で解く (fathomMany) (難しさ: 大)

**CBC 側**: 行+列 < 500 (`SIZEMINIBAB`、CbcSolver.cpp:10229–10241) で、`CbcGeneralDepth` から `ClpSimplex::fathomMany` (ClpSimplex.cpp:12478–) を呼ぶ。

**案**: まず、小さな問題で 1 ノードのうち LP 以外の時間の割合を測る。

### 木10. 費用を持つ整数列が少ない問題で、その列を先に分岐する (難しさ: 中、優先度低)

**CBC 側**: CbcModel.cpp:672–760 の `iType`。

**ENOMOTO 側**: 分岐の優先度の仕組みがない。

### 木: 既にある・不採用のもの

- **既にある**
  - 整数列の刻み (solver.rs:2655)。
  - 強分岐の片側の固定と両側の枝刈り。強分岐の子の整数解。子の下界。強分岐の予算の適応。ホットスタート。
  - 被約費用の固定 (局所・根)。`prune_queue` (`cleanTree` に当たる)。
  - orbital fixing・オービトープ、ダイビング 7 種、RINS/RENS/DINS/LB/FP/ALNS。
- **試して不採用**: `ENOMOTO_MIP_SB_PROP`、B2、B2m、L1、W9。CBC の「強分岐の中のプロービング」は `JJF_ZERO` で使われていない。
- **CBC でも既定外**: `CbcFathomDynamicProgramming`、`tightenVubs`、`goToEndInStrongBranching`。
- **ノードの選び方**: 今の HiGHS 流から CbcCompareDefault の形に変える根拠はない。

---

## 3. 切除平面とカットのループ

**時間の数字の出典**: `analysis/sepspeed_20261023.md` §3 (1 スレッド・根だけ・1 ラウンドあたり)。

### カット1. 根のループで毎ラウンド LU を捨てている (ENOMOTO だけの無駄、難しさ: 小)

**ENOMOTO 側** (確認済み)
1. 各ラウンドの最後に `remove_inactive_cuts()` (separation.rs:1589) → `delete_rows` → `rebuild()` (mip_lp.rs:250–258) が、`lu_cache = None`・`lu = None` にする。解いた直後の最適基底の LU はここで捨てられる。
2. 次のラウンドの tableau の分離 (`basis_inverse_row`、mip_lp.rs:1323–1348) が、分解し直す。
3. `add_rows` がもう一度 `rebuild()` し、求解でまた分解する。

1 ラウンドに作り直しが 2 回、分解が 2 回ある。CBC (`takeOffCuts` の後に CglGomory が自分で分解) の方が良いわけではなく、比べる途中で見つけた。

**案**
- 効いていないカットを外すのを、次のラウンドの分離の後、カットを足す直前に移し、削除と追加を 1 回の `rebuild` にまとめる。外す予定の行のスラックは基底にあるので、集約は正しい。
- 発展: 足すカットのスラックを基底に入れ、LU を作り直さずに拡張する (HiGHS の `HFactor::addRows`)。

### カット2. 分離器ごとの適応的な間引き (難しさ: 小〜中)

**CBC 側**
- `CbcModel::serialCuts` (CbcModel.cpp:11556–、判定 11616–11640、記録 11955–11981)。
  - 根で生成器を 3 回以上試し、3 回続けてカットが出なければ 5 パス休ませる。また外れたら休みを倍にする (最大 20)。
  - `boundStallAware` の生成器は、目的値がほとんど動かなければ外れと数える。
  - 既定で有効 (CbcParameters.cpp:980–990。442 問の比較で決めたと注記)。列が 500 未満の問題には使わない (`CbcCutAdaptiveSkipSettings`、CbcModel.hpp:134)。
- 根の終わり (CbcModel.cpp:11260–11450) に、生成器ごとのカットの数と目的値の伸びから、木での頻度 `howOften` を決める。
  - 0 本なら木で使わない。
  - 短いカットを作らなかった生成器は根だけ。
  - 伸びが 10% 未満なら深さ 5 おき。

**ENOMOTO 側**
- `separate()` (separation.rs:2996–) は毎ラウンド全分離器を回す。`sepprof` の数え上げは `ENOMOTO_MIP_DEBUG_SEPPROF` のときだけで、判定には使っていない。
- 木の `node_cut_round` (1810) も毎回、元の行・zerohalf・clique を回す。

**無駄の例**
- roll3000: 経路集約 2 種で 35.7 ms (分離 59.6 ms の 6 割) を使い、採られるのは 0.6 本。
- physiciansched6-2: 元の行 37.8 ms・zerohalf 54.5 ms で、どちらも採用 0。

**注意**
- roll3000 の tableau はカットを作れているのに、選択の閾値で落ちている (`cutloop_hl_20261022.md` §1)。tableau は間引きから外すか、最初の数ラウンドは間引かない。
- 採用数は、`remove_inactive_cuts` が外さずに残った本数 (CBC の numberCutsActive) で数える。

**前例**: `docs/branch_and_cut_plan.md:50` に方針として書いてあるが、まだ入っていない。

### カット3. 元の行の分離と経路集約の重複 (難しさ: 小)

**CBC 側**: `CglKnapsackCover` (CglKnapsackCover.cpp:114–121, 208–218) は締まった行だけを使う (木では |活動量 − 右辺| < 1e-5、根の 2 パス目以降は < 0.1)。

**ENOMOTO 側**
- 元の行のループ (separation.rs:3114–3160) は、全行の両側を毎ラウンド・毎ノードで試す。
- `path_aggregation_highs` (2276–) も、締まった行の始点だけでまず `generate_cuts` を呼ぶ (2470–2490)。
- そのため根では、締まった元の行のカットを 2 回作っている。

**案**
- (a) 根で経路集約が動くときは、元の行のループを「緩い行と extended cover」だけにする。
- (b) ノードでは、余裕/ノルム < 0.1 の行だけを試す。

roll3000 では元の行のカットが 1 ラウンドに 9.4 本採られるので、同じカットが経路集約からも出るかを先に確かめる。

### カット4. 分離器の並列化 (難しさ: 中)

**CBC 側**: `CbcModel::parallelCuts` (CbcThread.cpp:1955–)。

**ENOMOTO 側**: 並列なのは tableau (3486–3600) と従来の経路集約 (`gen_cuts_parallel`) だけ。元の行・zerohalf・clique・implied bound・HiGHS 流の経路集約は逐次。

**案**: 結果は逐次と同じにできる。

### カット5. zerohalf の前処理 (難しさ: 小〜中)

**CBC 側**: `Cgl012cut::get_parity_ilp` (Cgl012cut.cpp:940–1100) は次の削減をする。
- gcd で割ってから mod 2 にする。
- 奇数の係数がない行を消す。余裕の大きい行を消す。
- パリティの同じ行をハッシュでまとめ、余裕が最小の 1 本だけ残す。

**ENOMOTO 側**: `zerohalf.rs` の消去 (120–137) は、列ごとに `(0..ns).find` で軸を探し、全行のビットを見る (O(列 × 行) + XOR)。

**案**: 3 つの削減と、列から行への索引を足す。gcd で割ることでカットが強くなる行もある。

### カット6. tableau のカットの非零の予算 (難しさ: 小、前例に注意)

**CBC 側**: CglGomory.cpp:1160–1225, 2126–2130 は、作ったカットの非零の合計が `nReasonable` = max(5n, nnz/8 + min(10000, nnz+2m)) に達したら止める (`// Got enough`、確認済み)。

**ENOMOTO 側**: 行の数の上限 (separation.rs:3400–3405) だけ。mzzv42z は 1 ラウンドに候補 1295 本で 154 ms かかる。

**前例**
- `ENOMOTO_T_MIP_TAB_NNZ_BUDGET` と `TAB_HIGHS_RULES` は、30n20b8 の根の下界を 151 → 88〜109 に下げた。
- CBC の形は順に処理し、最初の密なカットも作るので、効いている 1 本は残るはず。まず 30n20b8 で確かめる。

### カット7. プールに入れるもの・プールの走査 (難しさ: 小)

**CBC 側**: `globalCuts_` には、LP に入れた大域的に妥当なカットと、根で外したカットだけが入る。

**ENOMOTO 側**: 根の各ラウンドで、選ばれなかった候補も全部 `add_to_pool` する (separation.rs:1146–1151)。`pool_violated` (1755–1771) が、取り出したノードごとにプール全体を走査する。

**案**: 既にある `ENOMOTO_MIP_POOL_SELECTED_ONLY` を評価する (未評価)。

### カット8. ノードでのカット 1 本ごとの長さの上限 (難しさ: 小)

**CBC 側**: CglGomory の `dynamicLimitInTree_` = max(50, n/40)。

**ENOMOTO 側**: `nca_dense` (solver.rs:2543–2559) は、平均が密になるとノードの分離を全部止める。

**前例**: C2a (根と木の両方、基底不等式) は +5.59% で不採用。木だけでカットの長さを制限する試みは記録にない。

### カット9〜11. 小さな無駄 (どれも難しさ: 小)

- **9.** カットの LP が失敗すると、止まった途中の基底から解き直している (separation.rs:1384–1393、背景の取り込み solver.rs:4305–4310)。カットを足す前の基底と DSE 重みを取っておき、`set_basis` で戻せば反復 0 で済む。
- **10.** `separate()` が呼ばれるたびに、LP の全行を複製する (separation.rs:3009、`row` は clone)。ノードの分離でも同じ。参照で渡せばよい (W12/W18 として挙がったが未対応)。
- **11.** 根のカットの LP に打ち切り値を渡していない (separation.rs:1327/1352/1373 は `cutoff: f64::INFINITY`)。CBC は `isDualObjectiveLimitReached` を使う。

### カット: 既にある・不採用のもの

- **既にある**
  - 重複カットのハッシュ。
  - 小さい係数の吸収と dynamism の制限 (C5)。違反していない・効き目の小さいカットを捨てる処理。
  - 平行なカットの判定 (CBC は隣の 5 本だけと比べるが、ENOMOTO は全部と比べる)。
  - `takeOffCuts`・カットの年齢。停滞での打ち切り (ENOMOTO の方が細かい)。
  - プロービングの含意によるカット、clique の auto/fallback。
- **不採用**: 整数係数化 (C4)、tableau の BTRAN を疎に・DSE 重みで行を選ぶ (C2b・C15)、`CMIR_DELTAS_HIGHS`。
- **対象外**: Twomir / RedSplit / LandP / OddHole / ResidualCapacity は下界を強める話で、無駄の削減ではないので扱わなかった。

---

## 4. 前処理

### 前1. プロービングで一般の行の係数を強める (難しさ: 中、新規)

**CBC 側**
- `CglPreProcess::preProcess` (CglPreProcess.cpp:110–130) は、プロービングを `setRowCuts(3)` で足す。
- `CglProbing.cpp:5893–6060` は、x_j を固定して伝播した後、触れた行の最大活動 `maxR` が上限より `gap` だけ小さければ、`Σa x <= b` を `Σa x − gap·x_j <= b − gap` に差し替える (`strengthenRow`)。
- クリーク行に限らず、ナップサック・big-M の行など任意の行に効く。

**ENOMOTO 側**
- `probe` (src/mip/mod.rs:2617) は、固定・締め付け・含意を集めるだけ。
- 係数を強めるのは次の 2 つだけ。
  - `tighten_coefficients` (mod.rs:3529): 1 行の活動量だけで判定。
  - `extend_clique_rows`: 集合詰め込み行だけ。
- `ENOMOTO_MIP_PROBE_REPRESOLVE` は既定外。

**案**: `Domain` は行の活動量を差分で保つ (`update_activity`) ので、プロービング直後に読むだけで判定できる。整数解の集合は変わらないので、後処理は要らない。

### 前2. 含意の境界を単体法に渡さない (難しさ: 中)

**COIN 側**
- `ClpSimplex::tightenPrimalBounds` (ClpSimplex.cpp:5568–5630) は、元が無限で締めた境界が 1e10 を超えれば無限に戻し、それ以外は 0.1 だけゆるめる。
- `testRedundant` (CoinPresolveUseless.cpp:40) は境界の写しで伝播し、行を落とすだけ。

**ENOMOTO 側**: `presolve.rs:1160–1200` の最終の伝播の境界を、そのまま単体法の箱制約にする。

**前例**: `analysis/nug08_irish_20260926_013500.md` 表 #10 に、反復 −10〜20% の見積もり。`improvement_history.md:7892` に「(D) 変更範囲が広いので見送り」。最小の形は「元が無限なら無限に戻す」だけで、LP に限る。

### 前3. 行・列の作業リストによる差分の前処理 (難しさ: 高)

**COIN 側**
- `CoinPresolveMatrix` の `rowsToDo_/colsToDo_/nextRowsToDo_` で、各規則は作業リストの行・列だけを見る。
- ClpPresolve.cpp:1209–1249 は、安い規則を変化がなくなるまで回す。
- 行列はその場で書き換える。

**ENOMOTO 側**
- `presolve.rs:525–1158` の外側ラウンドは、毎回全行・全列をなめ直す。
- 何か見つかると、`csr_from_rows` で A を作り直す。

**効果の見込み**: pds-100 は 1 ラウンド約 350 ms の作り直し (`improvement_history.md:8378–8384`)、irish は 45 ラウンドで 7.7 秒。ただし、まず `ENOMOTO_PROF_PRESOLVE` で段別の時間を測る (pds-100 の伝播は 38〜49 ms しかない)。

### 前4〜6. 整数最適化で閉じている規則を開ける (どれも難しさ: 低)

- **4. 費用 0 の強制列 (`do_tighten`)**
  - COIN の `CoinPresolveTighten.cpp` は整数列にも使う。
  - ENOMOTO の `forcingcol.rs` は、整数最適化では使わず (`presolve.rs:645`)、LP でも既定で無効 (LP では e226 などが 18〜31% 遅くなったため)。
  - 主側の議論なので、整数最適化でも正しい。
- **5. 不等式行の列シングルトン**
  - COIN の `CoinPresolveImpliedFree.cpp`・slack_singleton に当たる。
  - ENOMOTO の `ineqsingleton.rs` は、1 万行以上の問題だけで使う (`presolve.rs:476–479`)。整数最適化では、さらに `ENOMOTO_MIP_INEQ_SINGLETON` が要る。
  - 「40 問に当たらない」(`highsonly16_summary`) のは、1 万行の条件のため。
- **6. 集合分割の行の部分集合による固定**
  - CBC の `CglDuplicateRow::generateCuts12` に当たる。
  - ENOMOTO の `ENOMOTO_MIP_CLIQUE_EXTEND_FIX` は試作のまま未評価。mzzv42z で 111 列固定 (`mzzpre_20261024.md:82`)。

### 前7〜10. 固定費・特定の型 (優先度低)

- **7.** 範囲制約を 2 本に割ってから、まとめ直している (mod.rs:2045–2055 と `find_opposite_equality_pairs`)。COIN は `rlo_/rup_` で 1 本のまま持つ。
- **8.** `presolve_mip_depth` (mod.rs:2014–2058) は、かけ直しのたびに全行を BTreeMap の `LinearExpr` に詰め替える。ビット一致のまま減らせる。
- **9.** `CbcModel::tightenVubs` (big-M の連続列を LP で締める)。CBC でも既定外。
- **10.** gubrow (CoinPresolveDupcol.cpp:1610)。`SPARSIFY` に含まれる。

### 前: 既にある・不採用のもの

- **既にある**
  - dupcol は parallelcols のハッシュ (整数版は nw04 で悪化して保留)。
  - duprow は redundancy。doubleton/tripleton/subst は doubleton と aggregator v2。
  - forcing/useless は propagate。remove_dual_action は dualpropagate。
  - 双対固定、cliqueIt、係数の強化、目的の整数性、早めのかけ直し。
- **不採用**: dominatedcol (+17.8%)、smallcoeff (不安定化)、伝播での暗黙の整数の丸め (B13、+1.13%)。
- **CLP でも既定外**: twoxtwo・tightenDoubletons2。

---

## 次に測るときの進め方の案

1. **経路をほぼ変えない無駄の削減を 1 つの束にして、40 問 + 16 問で測る。** 対象はカット1・カット9・カット10・木5(a)・前8・LP3。WLC と同じ扱い。
2. **探索が変わるが手間の小さいものを、1 つずつ環境変数の試作にして比べる。** 対象は木1・木2・カット2・カット3・前4〜6。
3. **先に内訳の計測が要るものは、計測から始める。** LP4 は `ENOMOTO_MIP_XPROF`、前3 は `ENOMOTO_PROF_PRESOLVE`、木9 は LP 以外の時間の割合を測る。
