# 分枝切除法 (branch-and-cut) 実装提案

HiGHS v1.15.1 (`highs/mip/`、約 3 万行) と COIN-OR CBC / Cgl (master) のソースを読んだ結果と、
本リポジトリの現状の調査をもとにした実装計画。

---

## 1. 現状の整理

`src/mip.rs` (190 行) は深さ優先の素朴な分枝限定法で、各ノードで `solver::solve_lp` を最初から呼び直している。

| 項目 | 現状 | 問題 |
|---|---|---|
| ノード LP | 毎回 presolve → スケーリング → 双対単体法 (コールドスタート) | presolve の写像がノードごとに変わるので基底を引き継げない。計算時間の大半が無駄 |
| ノード選択 | DFS のみ | 下界と gap が計算できない |
| 分岐 | 最も小数的な変数 | 文献上、ランダム選択とほぼ同等の弱さ |
| 枝刈り | 緩和が `Optimal` 以外なら捨てる | `Unbounded` / `NotSolved` のノードを黙って捨てる (誤答の原因になる) |
| ノード上限 | 暫定解がないまま上限に達すると `Infeasible` を返す | 誤った状態を返す |
| 暫定解 | 丸めた x を実行可能性の確認なしで採用 | 誤答の原因になる |
| presolve | 整数性を一切考慮しない (`vtype` を参照する箇所がない) | MIP 全体に対する presolve としては使えない |

LP エンジン側に欠けているもの (`simplex.rs` / `slope_intercept_dual.rs`):

- **永続的なソルバー状態がない**: 基底・LU・DSE 重み・被約費用 `d` は関数の局所変数として捨てられる。
- **境界変更後の再最適化ができない**: `solve_slope_intercept_dual_from_basis` (`slope_intercept_dual.rs:813`) は基底の列集合しか受け取らず、DSE 重みを初期化し直す。
- **行の追加・削除 (カット) ができない**: `StdForm` と `CsrMat` / `CscMat` は不変。
- **出力されない情報**: 基底状態、双対値と被約費用、Farkas 光線。Farkas 光線は `infeasibility_certified` で計算はしているが返していない。
- **制御パラメータがない**: 目的値の打ち切り (cutoff)、利用者が指定する反復上限・時間上限。

すでにあって再利用できる部品:

- BTRAN 行 `BasisKernel::btran_row` (`basis_kernel.rs:164`)、`factorize_basis` (`basis_kernel.rs:286`)
- ラグランジュ下界 `lower_bound_from_basis` (`slope_intercept_dual.rs:921`)
- 協調キャンセル `cancel::with_token` (時間上限に使える)
- 活動量 (activity) による伝播の式 (`presolve/propagate.rs`)

---

## 2. HiGHS と CBC の比較と方針

| 観点 | HiGHS | CBC | 採用 |
|---|---|---|---|
| 全体構造 | `HighsSearch` (dive + backtrack)、`HighsDomain`、`HighsNodeQueue` に責務を分割 | 2.2 万行の `CbcModel.cpp` を大量のビットフラグで制御 | **HiGHS 型** |
| ノードの持ち方 | root からの境界変更スタック。基底は dive 中だけ親から継承し、キュー内のノードは持たない | 親との差分 (`CbcPartialNodeInfo`) を持ち、ノードごとに root からたどって再構築。子の LP は取り出した時に解く (遅延評価) | **HiGHS 型** (子はすぐ評価する) |
| 未処理ノードのキュー | 赤黒木 3 本 (best-bound / 0.5·lb + 0.5·estimate / suboptimal)。10 回に 1 回は best-bound を強制 | ヒープ 1 本。best-bound は毎回 O(n) で走査 | **HiGHS 型** (Rust では `BTreeSet` で書ける) |
| 境界伝播 | 行の活動量を差分更新し、変化が `capacityThreshold` を超えた行だけ再伝播。境界変更のたびに不動点まで回す | 弱い (`tightenBounds` を定期実行するのと probing のみ) | **HiGHS 型** (性能差の最大要因) |
| 分岐 | pseudocost + reliability (minrel = 8)、product score、inference / cutoff / conflict の副スコア、強分岐の反復予算は全体で管理 | pseudocost + reliability (numberBeforeTrust = 10)、`markHotStart` を使う強分岐、片側が不可能なら反対側に固定 | **HiGHS 型 + CBC の固定規則** |
| カット生成 | 汎用の `generateCut` (lifted cover と CMIR の良い方) を tableau 行の集約 (Gomory 相当)、path 集約、mod-k に適用 | Cgl の個別生成器 (Gomory、MIR2、Knapsack、FlowCover、TwoMIR、ZeroHalf、Clique、Probing) | **HiGHS 型**。生成器 1 つで多くのカットを賄える。CBC の Gomory の数値安全策 (`away`、条件数、dynamism) は取り入れる |
| カットプール | age、efficacy (viol / ‖a‖)、並列度 0.1 で除外、LP 内の age 上限 10 | slack が基底に入ったら削除するだけ | **HiGHS 型** |
| root カットループ | 停滞判定 (stall 3 回)、ラウンド数 2√log₂(木サイズ) | 目的値の履歴 7 回、最大 20 パス、生成器ごとの頻度を root での効果から決める | HiGHS 型。生成器ごとの頻度制御は CBC の考え方を簡略化して使う |
| ヒューリスティクス | trivial、Feasibility Jump、randomized rounding、RENS / RINS (sub-MIP)、feasibility pump、予算は LP 反復の 5% | FPump (root)、rounding、dive 系、RINS / RENS / DINS | 安価なものから順に入れる |
| その他 | 被約費用固定 (root の lurking bound)、conflict analysis、clique table、probing、restart、対称性 | 被約費用固定、`analyzeObjective` (目的係数の gcd から cutoff 増分を決める) | 後半フェーズで HiGHS 型を入れる |

---

## 3. 提案するモジュール構成

```
src/mip/
  mod.rs            solve_mip (入口)、MipResult、状態判定
  solver.rs         MipSolver: root 処理 → 探索ループ → 終了判定 (HighsMipSolver::run に相当)
  lp_relaxation.rs  LpRelaxation: 永続的な双対単体法の上に、境界の差分反映 (flush)、cutoff、
                    カット行の追加と削除、基底の保存と復元を載せる
  domain.rs         Domain: 境界変更スタック (理由つき)、backtrack、活動量の差分更新による伝播
  search.rs         Search: evaluate_node / branch / backtrack / dive / plunge
  node_queue.rs     NodeQueue: BTreeSet × 2 (lb 順、hybrid 順) + suboptimal 集合
  pseudocost.rs     Pseudocost: 上下別の平均、reliability、inference、cutoff
  branching.rs      reliability 分岐 + 強分岐
  heuristics/       trivial.rs, rounding.rs, fj.rs (Feasibility Jump), fpump.rs (Feasibility Pump), rens_rins.rs
  cuts/             pool.rs, generation.rs (lifted cover / CMIR), transformed_lp.rs,
                    tableau.rs (Gomory 相当), aggregator.rs
  redcost.rs        被約費用固定
  params.rs         許容誤差と各種上限 (mip_feasibility_tolerance = 1e-6, rel_gap = 1e-4, ...)
```

---

## 4. LP エンジン側に必要な拡張 (フェーズ 0、最重要)

ここが整わないと、上位の実装はどれも性能が出ない。

新しく `simplex::persistent::DualSimplexState` を作る (`slope_intercept_dual.rs` の局所状態を構造体に引き上げる)。

```rust
pub struct DualSimplexState { /* StdForm(可変 lb/ub, 行追加可), basis, nb_status, FtLu, dse_w, d, x_B ... */ }
impl DualSimplexState {
    fn new(lp: &MipLp) -> Self;                         // presolve なし、スケーリングのみ
    fn change_col_bounds(&mut self, cols: &[(usize, f64, f64)]); // 非基底の値と x_B を更新し、双対実行可能性を保つ
    fn add_rows(&mut self, rows: &[SparseRow]);          // slack を基底に入れ、LU を拡張または再分解
    fn delete_rows(&mut self, mask: &[bool]);            // basic な slack を持つ行だけ削除し、基底を詰める
    fn solve(&mut self, lim: &SolveLimits) -> LpStatus;  // Optimal | Infeasible | ObjectiveBound | IterLimit | TimeLimit | Unbounded
    fn change_costs(&mut self, c: &[f64]);                // Feasibility Pump 用 (目的の差し替えと復元)
    fn solve_primal(&mut self, lim: &SolveLimits) -> LpStatus; // 費用変更後は主実行可能性が保たれるので primal simplex で解き直す
    fn get_basis(&self) -> Basis;  fn set_basis(&mut self, b: &Basis);   // 列・行ごとの status
    fn snapshot(&self) -> Iterate; fn restore(&mut self, it: Iterate);   // 強分岐用 (LU を含めて保存と復元)
    fn col_value / row_value / col_dual(=被約費用) / row_dual / objective;
    fn btran_unit_row(&mut self, i: usize, out: &mut SparseVec);         // tableau 行 (Gomory 用)
    fn dual_ray(&self) -> Option<SparseVec>;                             // 不可能性の証明
    fn dse_weights(&self) -> &[f64];
}
```

設計上の注意:

- **ノード LP では presolve を走らせない**: `dualize`、`sifting`、連結成分分解、IPM との競争 (race) も使わない。presolve は MIP 全体に対して root で 1 回だけ行い (§7)、以後のノード LP はその縮約後の空間で解く。これで写像が固定される。
- **cutoff には真の費用で計算した下界を使う**: 双対単体法は費用を摂動して解くため、`lower_bound_from_basis` (真の `c` を使う) が cutoff を超えた時点で `ObjectiveBound` を返す。
- **境界変更後は primal の不可能性だけが生じる**: その状態から dual simplex の stage B を再開すればよい。変更前に非基底だった変数は新しい境界に移す。
- **ベンチマークで退行がないことを確認する**: 既存の LP ベンチマーク (Netlib / Mittelmann) を回し、LP 経路に退行がないことを見る。新しい構造体は既存の関数から呼ぶだけの形にし、ロジックを二重に持たない。

### 4.1 Feasibility Pump (段階 4)

HiGHS (`HighsPrimalHeuristics::feasibilityPump`、約 120 行) と同じ形で実装する。

- **呼ぶ時点**: root のカットループと root ヒューリスティクスが終わった時点で、暫定解がまだ無ければ 1 回呼ぶ。木の探索中には呼ばない。CBC は root の最初に呼ぶが、カット後の LP 解の方が丸めやすいので HiGHS の順序に従う。
- **反復** (LP 解に分数の整数変数が残っている間):
  1. 整数変数を丸める。しきい値は 0.4〜0.6 の乱数にする。
  2. 固定した値を局所 `Domain` に入れて伝播する (段階 2)。
  3. 丸めた点が以前と同じなら循環とみなし、ランダムに 10 変数を反転する (最大 2 回)。それでも循環すれば終了する。
  4. LP 解と丸めた点を結ぶ線分上で実行可能な点を探す丸め (line search rounding) を試す。成功すれば暫定解にして終了する。
  5. 目的関数を丸めた点への L1 距離 (係数 ±1 に微小な乱数を加えたもの) に差し替え、`solve_primal` で解き直す。lock が片側 0 の変数は係数を 0 にする。
- **作業量の上限**: LP 反復の合計を `1000 + 5 × root LP の平均反復数` までにする。LP が 0 反復で終わったら打ち切る。
- **LP の扱い**: LP は `DualSimplexState` を複製して使う。本体の基底と費用を壊さないため。
- **後の改良候補**: 目的値をある程度保つ変種 (objective FP) と、終了時に整数を固定して小さな MIP で仕上げる処理 (CBC の `smallBranchAndBound` に相当、RENS の部品で実現できる)。

---

## 5. 段階的な実装計画

各フェーズの終わりに、MIPLIB 2017 の easy から選んだ小〜中規模の約 30 問 (MPS は `highspy` で読む。既存の `benchmark_highs.py` の流れを使う) で次の 2 点を測る。

- 正しさ: 最適値が HiGHS と一致すること
- 性能: 時間の幾何平均を HiGHS と比べる

| フェーズ | 内容 | 主な参照元 | 見積もり (Rust 行数) |
|---|---|---|---|
| **0'** | MIP presolve: 既存の LP presolve に整数変数用の分岐を追加する (§7)。段階 0 と並行して進められる | Achterberg et al. 2020、PaPILO | 600–900 |
| **0** | `DualSimplexState` (§4)。単体テストでは、境界を変更して warm start した結果がコールドスタートと一致することを確かめる | HighsLpRelaxation、Osi の hot start | 1,000–1,500 |
| **1** | 木探索の骨格: `Domain` (変更スタックと backtrack のみ)、`Search` (dive + backtrack、基底の継承)、`NodeQueue` (best-bound / hybrid)、cutoff、gap、上限 (時間 / ノード / gap)、状態の正しい分類、暫定解の実行可能性確認。この時点の分岐は pseudocost のみ | HighsSearch、HighsNodeQueue | 1,500 |
| **2** | 境界伝播: 行活動量の差分更新、整数境界の丸め、連続変数は改善 30% 以上の場合だけ採用、`capacityThreshold` | HighsDomain | 1,200 |
| **3** | reliability 分岐: 強分岐 (`snapshot` / `restore`、全体反復予算)、product score、inference / cutoff の副スコア、片側が不可能なら反対側に固定 | HighsPseudocost、`selectBranchingCandidate`、CbcNode | 800 |
| **4** | 安価な主ヒューリスティクス: trivial、simple / randomized rounding、Feasibility Jump、**Feasibility Pump** (§4.1)。目的が整数値をとる場合の cutoff 増分 | HighsPrimalHeuristics (`feasibilityPump`)、HighsFeasibilityJump、CbcHeuristicFPump、CbcModel::analyzeObjective | 1,100 |
| **5** | root のカットループとカットプール: `generateCut` (lifted cover / CMIR)、TransformedLp (境界と VUB の代入)、tableau 分離器 (Gomory 相当)、age / efficacy / 並列度、stall 判定、basic なカット行の削除 | HighsCutGeneration、HighsTableauSeparator、HighsCutPool、HighsSeparation、CglGomory (安全策) | 2,500 |
| **6** | 被約費用固定 (root の lurking bound とノード)、RENS / RINS (再帰 MIP)、MIP 専用の縮約 (§7.3) | HighsRedcostFixing、HPresolve | 1,500 |
| **7** | conflict analysis (dual proof)、clique table、probing、path / mod-k 分離器、restart、対称性 | ConflictSet、HighsCliqueTable、HighsImplications | 4,000 以上 |

期待される効果は概ね **0 ≫ 2 ≈ 5 > 3 > 1 > 4 > 6 > 7** の順。ただし 1 は他のすべての前提になる。

---

## 6. 先に決めておきたいこと

1. ~~MIP の presolve をどうするか~~ → **既存の LP presolve に整数変数用の分岐を追加する形に決定** (§7)。
2. **並列化**: 最初は単一スレッドで作る。HiGHS の並列 worker は後から追加された機能で、骨格には不要。
3. **ベンチマーク集合**: MIPLIB 2017 の benchmark / easy から 30 問程度を `benchmarks/miplib/` に置く。容量が大きいのでリポジトリには置かず、取得スクリプトだけにする案。
4. **Python API の拡張**: `SolveResult` に `best_bound`、`mip_gap`、`node_count`、`TimeLimit` / `NodeLimit` の状態を追加し、`solve()` に `time_limit`、`mip_rel_gap`、`node_limit` を渡せるようにする。

---

## 7. MIP presolve: 既存 LP presolve への整数分岐の追加

方針: 新しい presolve を作らず、`presolve::run_extended` に整数性の情報 `is_int: &[bool]` を渡す。各縮約の中に「整数変数の場合」の分岐を足す。MIP の root で 1 回だけ実行し、ノード LP はその縮約後の空間で解く。

### 7.1 共通の変更

- **整数性を引き継ぐ**: `build_a_g` から縮約後の列まで `is_int` を伝える。列の削除・置換・併合のたびに更新し、`ExtendedPresolveResult` に縮約後の `is_int` を持たせる。
- **境界を丸める**: 整数列の境界を更新するときは必ず `lb ← ceil(lb − feastol)`、`ub ← floor(ub + feastol)` で丸める。共通関数を 1 つ用意し、全縮約から呼ぶ。丸めた結果が `lb > ub` なら実行不能とする。
- **スケーリング**: 整数列には列スケーリングを掛けない (係数 1 に固定)。行スケーリングはそのまま使ってよい。列スケールを掛けると、縮約後の空間で「x が整数」という条件が「x' が 1/s の倍数」に変わってしまうため。LP ソルバー内部のスケーリングは別の層なので問題ない。
- **postsolve**: 主の値の復元だけで十分 (MIP では元の空間の双対値は不要)。ただし整数列を置換で消した場合、復元した値が整数になることを縮約の時点で保証しておく (§7.2 の条件)。
- **MIP モードの切り替え**: `is_int` がすべて false なら、今の LP の挙動と完全に一致させる。既存の LP ベンチマークで出力のハッシュ (`presolve_output_hash`) が変わらないことで確かめる。

### 7.2 縮約ごとの扱い

| 縮約 (ファイル) | 整数変数の場合 |
|---|---|
| propagate / ineqsingleton / rowsingleton / foldfixed | そのまま使える。導いた境界と固定値を丸めるだけ。`rowsingleton` の固定値が整数でなければ実行不能 |
| dualfix / dualpropagate / forcingcol | 境界への固定はそのまま有効 (丸め済みの境界なので整数)。dualpropagate の「含意自由」判定を列の消去に使う場合は、下の freevar と同じ制約に従う |
| redundancy / smallcoeff | 行だけの操作なので有効。smallcoeff で右辺を調整した結果、整数行の右辺の性質が変わる点だけ注意する |
| colsingleton / aggregator / freevar (列を置換で消すもの) | **消す列が連続変数のときだけ**適用する。整数列を消せるのは、残りの列がすべて整数で、係数比と右辺がすべて整数 (例: 係数 ±1) になり、復元値が自動的に整数になる場合に限る |
| doubleton (`a·x + b·y = c` で x を消す) | x が連続なら従来どおり。x が整数なら、y も整数で `b/a` と `c/a` が整数の場合だけ適用する。どちらも整数で条件を満たさない場合は、x と y の境界を (一次不定方程式の解の構造から) 強める処理だけにする |
| parallelcols (平行な列の併合) | 同じ型の列どうしで、係数比が ±1 の場合だけ併合する (併合後の列も整数になり、postsolve で整数に分割できる)。整数と連続が混ざる場合や比が ±1 以外の場合は、併合せずに支配関係による固定だけ行う |
| scaling | §7.1 のとおり、整数列は列スケール 1 |
| 現在未使用のもの (dominatedcol / parallelrows / rowdominance / sparsify / stuffing) | 有効化する際に同じ規則で分岐を入れる。stuffing は連続列だけを対象にする |

### 7.3 MIP にしかない縮約 (段階 6 以降で追加)

- **係数の強化 (coefficient tightening)**: `a_j > maxact − rhs` を満たす 0-1 変数の係数を下げる。LP 緩和が強くなり、効果が大きい。
- **整数行の右辺の丸め**: 係数と変数がすべて整数の行で、`rhs ← floor(rhs + feastol)` とする。gcd でも割る。
- **暗黙整数の検出**: 等式行の残り 1 つの連続変数が、他が整数なら整数値しかとらない場合に整数扱いにする。分岐対象にはしないが、カット生成に使う。
- **probing / clique 抽出**: 段階 7 で domain 伝播の部品を使って実装する。

---

## 9. 実装の状況 (2026-10-06)

| 段階 | 状況 | 実装 |
|---|---|---|
| 0 | 一部完了 | MIP の LP は既存の傾き・切片二段解法を使う (`simplex/mip_lp.rs`)。前処理なしの標準形を保持し、境界の変更は平行移動と右辺の差分更新で反映、直前の最適基底から warm start。二段解法本体には外部からの打ち切り条件 (反復上限・目的値・時刻) と、最適基底の LU の受け渡しを追加。分枝限定法専用に書いた単体法 (`mip/lp.rs`) も `ENOMOTO_MIP_LP=own` で選べるが、blend2 / qnet1 で誤答するので既定にしない。残り: 求解ごとの準備処理 (PRICE 用の行列など) の持ち越し |
| 0' | 完了 | `presolve::run_extended_mip`。列を消す縮約は連続列だけ、整数列の境界は丸める、双対の議論に基づく縮約とスケーリングは行わない。後処理した解を元の問題で検査し、だめなら前処理なしで解き直す |
| 1 | 完了 | `mip/solver.rs`、`mip/queue.rs`。plunge、下界順と hybrid estimate 順の待ち行列、打ち切り、時間・ノード・ギャップの上限 |
| 2 | 完了 | `mip/domain.rs`。活動量の差分更新、整数の丸め、連続変数は 30% 以上の改善のときだけ |
| 3 | 完了 | pseudocost + 強分岐 (reliability 8、全体の反復予算、片側が打ち切りなら固定) |
| 4 | 完了 | 単純丸め、固定と伝播による丸め、Feasibility Pump、Feasibility Jump |
| 5 | 一部完了 | 根の切除平面ループ (CMIR、拡張カバー、元の行と tableau 行)。ノードでの分離は実装したが遅くなる問題が多く既定では無効 (`ENOMOTO_MIP_NODE_CUTS=1`) |
| 6 | 完了 | 被約費用固定 (根の情報で大域的に、各ノードで局所的に)、RENS / RINS (サブ MIP) |
| 7 | 未着手 | conflict analysis、clique、probing、path / mod-k、restart、対称性 |

検証: 乱数 MIP (`scripts/mip_fuzz_vs_highs.py`) で HiGHS と最適値が一致することを各変更で確認。
MIPLIB 2017 の 40 問の比較は `scripts/run_miplib_benchmark.py`。

既知の課題:
- 強分岐の LP 反復が多い (10teams など退化の強い問題で、1 回の LP が 100〜500 反復)
- 退化の強い問題で暫定解が見つかるのが遅い (10teams など)
