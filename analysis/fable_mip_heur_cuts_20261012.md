# MIP: 主ヒューリスティクスと根のカット生成の弱点の調査 (2026-10-12)

`benchmarks/miplib_results_16t_60s_20261012.md` (16 スレッド 60 秒) で HiGHS に劣る問題について、
(1) 暫定解が見つからない・悪い (30n20b8, neos-1456979, bppc4-08, timtab1, h80x6320d, supportcase26, neos-911970,
graph20-20-1rand, assign1-5-8)、(2) ノード数が HiGHS より桁違いに多い (neos-860300, h80x6320d, binkar10_1, qnet1,
timtab1, blend2, p0201) の 2 点を、コードの読解と計測で調べた。計測は 4 vCPU の VM で **threads=1、60 秒、3 問同時**
(HiGHS も threads=1)。時間は同時実行のばらつきを含むので、目安として読むこと。ログは `ENOMOTO_MIP_LOG=1` の verbose
出力 (`MIP: cut round ...`, `MIP: root heuristic ...`, 周期的な `nodes ... heur_iters ...` 行) を使った。一時的な
計測コードは入れておらず、作業ツリーはこの報告だけが差分。

結論を先に書くと:

- **根のカットは「1 ラウンドに採る本数が少なすぎる」「効いていないカットを毎ラウンド即座に外す」「25 ラウンド・停滞 10 で
  打ち切る」の 3 点で収束が遅く**、多くの問題で 25 ラウンドの上限に当たったまま終わる (p0201, qnet1, timtab1, supportcase26,
  assign1-5-8, misc07, mik-250)。HiGHS は同じ問題で数千本の候補から数百本を LP に入れて根の下界をずっと高くする
  (timtab1: 485k vs 569k、qnet1: 15606 vs 16022、neos-860300: 2532 vs 3050、h80x6320d: 5900 vs 6310)。
- 分離器の欠けでは、**clique カット (既定オフ) と implied bound カットが無い** ことが graph20-20-1rand (-37 のまま。HiGHS は
  -21) と neos-1456979 (154 のまま。HiGHS は 171) の根の下界が全く動かない直接の原因と考えられる (後者は推定)。
- **主ヒューリスティクスの最大の問題は木の中の LNS (RINS/ALNS) の予算の数え方**。予算 `総 LP 反復/20 + 10000` に対し、
  根のサブ MIP の反復 (`heur_iters`) を引かずに数えるので、根で数万反復を使った問題では**木の中で RINS・ALNS・ランダム丸めが
  60 秒間ほぼ一度も走らない** (h80x6320d: heur_iters 65k vs 予算 15k、timtab1: 63k vs 56k、assign1-5-8: 58k vs 43k、
  graph20: 56k)。`ENOMOTO_MIP_LNS_TIME=1` (時間の予算) にするだけで h80x6320d 6860 → 6598、timtab1 1074786 → 996666。
- 根のヒューリスティクスは**順番と予算が固定**で、30n20b8 では根のヒューリスティクスだけで 40.6 秒 (RENS 14.9 秒など) 使って
  解なし。再スタート後は失敗したヒューリスティクスを全部飛ばす (`skip_heurs`) ので、再スタートで問題が締まっても解を探さない。

## 1. 計測 (threads=1、60 秒、ours vs HiGHS 1.15.1)

### 1.1 根の下界と最終結果

「HiGHS 根 (最終)」は HiGHS のログでノード数 0 の最後の行の下界 (再スタート後のカットを含む。被約費用固定・サブ MIP の
解による締め付けを含む)。ours の「カット後」は最初の求解の根のカットループの終了時の値 (再スタート後の値は括弧内)。

| 問題 | 根 LP | ours カット後 (ラウンド, 採用/候補) | HiGHS 根 (最終) | ours 根の所要 [s] | ours 最終 (目的値 / 下界 / ノード) | HiGHS 最終 (目的値 / 下界 / ノード) |
|---|---:|---|---:|---:|---|---|
| 30n20b8 | 43.34 | 143.61 (12, 142/3896) | 129.72 | 40.60 | 解なし / -inf / 61 | 553 / 130.0 / 0 |
| assign1-5-8 | 183.36 | 186.62 (25, 40/5481) | 193.77 | 1.96 | 214 / 193.84 / 23357 | 212 / 197.0 / 17459 |
| binkar10_1 | 6637.19 | 6668.53 (5, 36/715; 再スタート 3 回後 6717.65) | 6701.23 | 1.58 | 6742.20 / 最適 / 2089 | 6742.20 / 最適 / 1547 |
| blend2 | 6.9157 | 6.9162 (6, 7/24; 再スタート後 6.9172) | 7.1384 | 0.15 | 7.599 / 最適 / 391 | 7.599 / 最適 / 67 |
| bppc4-08 | 51.59 | 51.59 (7, 37/7034) | 52 | 4.26 | 61 / 51.59 / 9760 | 53 / 52.0 / 9055 |
| graph20-20-1rand | -37 | -37 (2, 96/5610) | -21.36 | 11.44 | -8 / -37 / 937 | -8 / -21 / 2 |
| h80x6320d | 5325.16 | 5900.31 (20, 554/48275) | 6341.16 (ヒューリ前 6309.7) | 19.63 | 6860.66 / 6018.19 / 3665 | 6382.10 / 最適 / 15 |
| mik-250-20-75-4 | -61651.2 | -54576.0 (25, 197/12332) | -53426.8 | 4.02 | 最適 / 3371 | 最適 / 4201 |
| misc07 | 1415 | 1533.98 (25, 251/6725) | 1480 | 0.90 | 最適 / 15279 | 最適 / 6640 |
| neos-1456979 | 154 | 154 (6, 283/7239) | 171 | 14.47 | 266 / 154 / 2222 | 205 / 171 / 100 |
| neos-860300 | 2035.36 | 2532.53 (23, 157/7022; 再スタート後 2554.16) | 3172.09 (サブ MIP 直後 3050.0) | 7.49 | 最適 / 722 | 最適 / 1 |
| neos-911970 | 23.26 | 45.76 (21, 208/16356) | 52.74 | 1.47 | 60.69 / 52.58 / 14385 | 54.76 / 最適 / 1370 |
| p0201 | 7125 | 7341.54 (25, 69/5984; 再スタート後 7370.20) | 7420.30 | 0.30 | 最適 / 61 | 最適 / 5 |
| qnet1 | 14274.1 | 15606.2 (25, 134/6213; 再スタート後 15713.9) | 16022.0 | 2.32 | 最適 / 3 | 最適 / 1 |
| supportcase26 | 1288.10 | 1347.70 (25, 388/21823) | 1401.92 | 12.42 | 1946.56 / 1386.85 / 20303 | 1772.51 / 1431.28 / 13943 |
| timtab1 | 162691 | 485313 (25, 368/74174) | 609536 (ヒューリ前 569441) | 1.58 | 1074786 / 586289 / 17147 | 764774 / 最適 / 9046 |

読み取れること:

- **25 ラウンドの上限で止まる問題が 7 問** (assign1-5-8, mik-250, misc07, p0201, qnet1, supportcase26, timtab1)。どれも
  最後のラウンドまで下界が伸びている (例: timtab1 は 22 ラウンド目まで毎回 1-3% 伸びる、p0201 は 24 ラウンド目まで伸びる)。
- **候補のうち採る本数が極端に少ない**: p0201 は 130-480 本の候補から毎ラウンド 1-4 本、assign1-5-8 は 25 ラウンドで 40 本、
  timtab1 は 2300-3900 本から 10-30 本。上限は 50 本 (`ENOMOTO_T_MIP_ROOT_MAX_CUTS`) なのに届かない。原因は 1.3 節。
- 根の下界が全く動かない 3 問: graph20-20-1rand (-37)、neos-1456979 (154)、bppc4-08 (51.59。HiGHS も 52 なのでこれは
  主側の問題)。
- 30n20b8 は根の下界は HiGHS より良い (143.6 vs 129.7) が、根のヒューリスティクスに 40.6 秒使って解なし。

### 1.2 最初の暫定解

| 問題 | ours 最初の解 (値, 時刻 s, 手法) | ours 根終了時 | HiGHS 最初の解 (手法, 値, 時刻 s) | HiGHS 根終了時 |
|---|---|---|---|---|
| 30n20b8 | なし (根 40.6 秒) | なし | サブ MIP, 553, 49.6 | 553 |
| assign1-5-8 | 248, 1.92, trivial | 228 (clique) | ランダム丸め, 255, 0.0 | 212 |
| binkar10_1 | 8549.7, 0.00, ランダム丸め | 6754.85 (再スタート 3 回) | FJ, 2011758, 0.0 | 6747.31 |
| blend2 | 7.717, 0.22, root reduced cost | 7.717 | サブ MIP, 7.599, 1.3 | 7.599 |
| bppc4-08 | 290, 2.41, trivial | 290 | FJ, 5159, 0.1 | 57 |
| graph20-20-1rand | 0, 0.00, FJ/自明 | -8 (RINS, 11.4 s) | FJ, -1, 0.1 | -8 |
| h80x6320d | 17790, 0.00, FJ | 6860.66 (RENS, 16.4 s) | FJ, 16952, 0.2 | 6382.10 (= 最適, 11 s) |
| mik-250-20-75-4 | 0, 0.04, 単純丸め | 0 | FJ, 0, 0.0 | -52301 |
| misc07 | 2810, 0.48, trivial | 2810 | FJ, 4360, 0.1 | 2915 |
| neos-1456979 | 266, 14.39, RENS | 266 | サブ MIP, 206, 12.3 | 206 |
| neos-860300 | 6875, 3.66, trivial | 4321 | FJ, 6118, 2.6 | 3201 (= 最適) |
| neos-911970 | 91.36, 0.00, FJ | 75.81 (RENS) | ランダム丸め, 203, 0.1 | 56.92 |
| p0201 | 9770, 0.29, trivial | 7995 (RINS) | FJ, 11340, 0.0 | 7615 |
| qnet1 | 23053, 0.00, ランダム丸め | 16029.7 | FJ, 51197, 0.0 | 16029.7 |
| supportcase26 | 2201.9, 0.60, trivial | 2180.1 (RENS) | ランダム丸め, 2201.9, 0.0 | 2048.5 |
| timtab1 | 1074786, 1.09, fractional diving | 1074786 | サブ MIP, 816264, 1.9 | 804458 |

HiGHS は根のサブ MIP (RENS/RINS、`L`) で最適解に近い解を根で得ている問題が多い (h80x6320d, neos-860300, timtab1,
supportcase26, bppc4-08)。こちらのサブ MIP は同じ段階で質の悪い解 (h80x6320d 6860、timtab1 解なし、bppc4-08 解なし) で、
その後の木で改善できていない (1.4 節)。

### 1.3 根のカットループの中身 (代表例)

p0201 (ours、カット 25 ラウンド、0.29 秒):

```
cut round 0: 3 cuts (of 372) obj 7153.4   round 1: 3 (of 230) 7176.1   round 2: 3 (of 480) 7185.7
... round 23: 2 (of 139) 7337.9   round 24: 2 (of 163) 7341.5   -> 25 ラウンドで打ち切り
HiGHS: 2947 cuts / 39 in LP で 7399.0、再スタート後 7420.3
```

neos-1456979 (ours、6 ラウンドで停滞判定):

```
round 0..5: 44-50 cuts (of 1110-1272), cut rows mean nnz 2, obj 154.0 のまま
候補の内訳 (ENOMOTO_MIP_DEBUG_SEP): 元の行 129 / zerohalf +32 / clique 0 (オフ) / 経路集約 +30 / tableau +230 (平均 900 非零)、
面の頂点 2 つで ×3。採られるのは非零 2 本の (効き目 = 違反量/ノルム が大きい) カットばかりで、下界は動かない。
HiGHS: 1911 cuts / 489 in LP で 154 -> 171 (12 秒)
```

30n20b8 (ours): 12 ラウンドで 143.6 (HiGHS 129.7)。tableau 行のカットが平均 1000-1900 非零で、1 ラウンド 1.2 秒 (分離) +
LP 最大 6280 反復。根のカットは 6.4 秒で済んでいる。

timtab1 (ours): 候補 2300-3900 本/ラウンド (ほとんど元の行の CMIR と経路集約)、採用 10-30 本、25 ラウンド 1.0 秒で 485k。
プールは 17987 本で非零上限 200000 に当たり、以後の候補は捨てられる。

### 1.4 木の中のヒューリスティクスの予算 (ログの `heur_iters` と予算の比較)

予算は `budget = 総 LP 反復/20 + 10000` (solver.rs:2107)、条件は `heur_iters - heur_iters_root < budget` で
`heur_iters_root` は既定 0 (solver.rs:1585)。

| 問題 | 根の終了時の heur_iters | 60 秒時点の総 LP 反復 → 予算 | 60 秒時点の heur_iters | 木の中の LNS 呼び出し |
|---|---:|---:|---:|---|
| h80x6320d | 65,035 | 101,448 → 15,072 | 65,303 | 0 回 (RINS は根で 1 回のみ) |
| timtab1 | 30,735 | 920,110 → 56,005 | 63,193 | ALNS 2 回 (RINS 1000 ノードで 31k 反復を使い、以後予算切れ) |
| assign1-5-8 | 57,721 (5.9 s) | 658,089 → 42,904 | 58,481 | 2 回 (5.9 秒まで)、以後なし |
| supportcase26 | 39,110 | 631,863 → 41,593 | 48,531 | 1 回 (node 4)、58 秒に 1 回 |
| graph20-20-1rand | 56,320 | 147,166 → 17,358 | 57,120 | 0 回 |
| neos-911970 | 19,633 | 937,310 → 56,865 | 135,454 | 数回 (RINS で 75.8 → 60.7、33 秒で 100k 反復を使って以後なし) |

ダイビングの割当 `dive_iters < 0.05·(succ+1)/(calls+1)·node_iters + 1000` (solver.rs:2204) も、成功しない問題では
node_iters の 2.5% に縮み、60 秒で 1,100-1,700 反復しか使えていない (全問題で `dive_iters` が 1-3k)。

### 1.5 設定を変えた実験 (60 秒、threads=1)

木の中の LNS の予算 (`ENOMOTO_MIP_HEUR_BUDGET_TREE=1`: 根の分を引く、`ENOMOTO_MIP_LNS_TIME=1`: 経過時間の 2-10% の時間予算で
50 ノードおき、`ENOMOTO_MIP_RENS_DIVE=1`: 木の RENS をダイビング式に):

| 問題 | 既定 | HEUR_BUDGET_TREE | LNS_TIME | 両方 | RENS_DIVE | HiGHS |
|---|---:|---:|---:|---:|---:|---:|
| h80x6320d | 6860.66 | 6691.01 | **6597.79** | 6702.11 | 6847.57 | 6382.10 |
| timtab1 | 1074786 | 1074786 | **996666** | 967700 | 1011528 | 764774 |
| bppc4-08 | 61 | 60 | EXP_BPPC_LNS | EXP_BPPC_BOTH | EXP_BPPC_RD | 53 |
| supportcase26 | 1946.56 | EXP_SC26 | | | | 1772.51 |
| neos-911970 | 60.69 | EXP_N911 | | | | 54.76 |
| assign1-5-8 | 214 | EXP_A158 | | | | 212 |
| graph20-20-1rand | -8 | EXP_G20 | | | | -8 |

(表の EXP_ は計測が終わり次第埋める。timtab1 の LNS_TIME では 40-43 秒に mutation・crossover が 4 回続けて改善している
ログが取れており、「呼べば効く」ことの証拠になっている。HEUR_BUDGET_TREE だけでは timtab1 は 3 回しか呼べず改善なし:
RINS の 1000 ノードのサブ MIP 1 回で予算の半分以上を使う。)

暫定解のない 2 問 (30n20b8, neos-1456979): `ENOMOTO_MIP_RESTART_ALL_HEURS=1`、`ENOMOTO_MIP_NOINC_HEUR=1`、
`ENOMOTO_T_MIP_NOINC_RENS_TRIES=3`: EXP_NOINC

根だけ (node_limit=1) でカットの設定を変えた結果 (根のカットループ終了時の下界と所要時間):

EXP_CUTS

## 2. コードの構造 (読んだ範囲のまとめ)

### 2.1 根の切除平面ループ (`src/mip/separation.rs` `root_cut_loop`, 129-433 行)

| 項目 | 現状 (既定値) | 箇所 |
|---|---|---|
| ラウンド数の上限 | 25 (サブ MIP は 5) | separation.rs:148 `ENOMOTO_T_MIP_CUT_ROUNDS` |
| 時間の上限 | 時間上限の 10% (CPU 時間) | separation.rs:149, 177 |
| 1 ラウンドの LP の反復上限 | 10·root_iters + 10000、時間は全体の 2% (最低 1 秒) | separation.rs:260-269 |
| 1 ラウンドに加える本数 | 最大 50 (`ENOMOTO_T_MIP_ROOT_MAX_CUTS`) | separation.rs:218 |
| 選択 | `select_cuts_hybrid`: 質 = 効き目 + 0.1·目的平行度 + 0.1·整数台割合。**質が最良の 50% 未満は捨て** (`ENOMOTO_T_CUTSEL_MIN_REL`)、採ったカットとの平行度 > 0.3 (良いカットは 0.7) は捨てる | separation.rs:2139-2172 |
| 停滞判定 | SCIP 流: 相対改善 ≤ 1e-4 かつ分数列数が 0.9 倍未満に減らなければ stall+1 (目的値も分数列数も全く減らなければ +2)。stall ≥ 10 で終了 | separation.rs:355-394 |
| 効いていないカットの除去 | **毎ラウンド**、論理変数が基底で余裕のあるカットの行を全て LP から外す (`remove_inactive_cuts`、年齢なし。`ENOMOTO_T_MIP_ROOT_CUT_AGE=0`) | separation.rs:396-407, 878-900 |
| プール | 候補は全てプールへ (重複はハッシュで除去)。非零の上限 max(20·nnz, 200000) に達すると**黙って捨てる** | separation.rs:437-463 |
| 面の頂点での分離 | 既定 `int2`: 退化していれば副目的 2 方向の頂点でも分離 (LP を 2 回 clone して解く)。候補が 3 倍になる | separation.rs:1385-1508 |

分離器 (`separate`, separation.rs:1512-1961)。候補は 1 本の集約行 (向きごと) につき `generate_cuts` の最良 1 本 (`keep_best`):

| 分離器 | 対象 | 備考 |
|---|---|---|
| CMIR (`cuts.rs:256`) | 元の行 (両向き)、経路集約行、tableau 行 | δ 候補は「境界の内側にある整数列の係数」最大 8 個とその 1/2, 1/4, 1/8 (cuts.rs:266-291)。δ=1 は候補に入らない |
| lifted flow cover (`cuts.rs:773`) | 連続変数を含む元の行・経路集約行 | tableau 行では既定で使わない (`ENOMOTO_MIP_TAB_FC`) |
| extended cover (`cuts.rs:600`) | 連続変数を含まない元の行のみ | lifted cover (`cuts.rs:1161`) は既定オフ (`ENOMOTO_MIP_LIFTED_COVER`) |
| zerohalf (`zerohalf.rs:39`) | **元の行だけ** (カットの行・集約行は対象外)、最大 100 本 | mod-k 一般化なし |
| 経路集約 (HiGHS 移植 `path_aggregation_highs` + 旧 `path_aggregation`) | LP 行 (カット行を含む) | 2 系統で重複候補 |
| tableau 行 (Gomory 相当) | 基底の分数整数列 200 + 0.1·min(m, nint) 行、f(1-f) 順 (`dse_weight` は 1 固定) | 集約行から CMIR のみ |
| clique (`clique.rs:111`) | — | **既定オフ** (`ENOMOTO_MIP_CLIQUE_CUTS`、separation.rs:1679) |
| implied bound cuts | — | **無い** (HiGHS の `HighsImplications::separateImpliedBounds` 相当なし。プロービングの含意 `p.conflict_edges` は clique グラフの辺にしか使われない) |
| knapsack cover の持ち上げ | — | extended cover のみ。HiGHS は CMIR の前に lifted cover を試して効き目で選ぶ |

`finish_cut` (separation.rs:1965-2097): 小係数 (≤ 1e-4·cmax) を境界で吸収、係数の締め付け、dynamism > 1e6 で破棄、最大係数
1 にスケール、効き目 < 1e-5 で破棄、暫定解を切るなら破棄。整数台のカットの整数スケール・右辺切り下げは不採用 (C4)。

### 2.2 主ヒューリスティクスの枠組み

根 (`src/mip/solver.rs:1459-1580`)。固定の順に: simple rounding → ZI round → trivial (暫定解なし) → randomized rounding (3 回)
→ interior rounding → root reduced cost (サブ MIP) → LP face (サブ MIP) → RENS (サブ MIP; 暫定解なしなら固定率 0.8 の
ダイビング式で残り時間の 40%・最大 20 秒) → RENS (カット前 LP) → feasibility pump (暫定解なし; 反復 1000 + 5·root_iters)
→ fractional / vector-length / coefficient diving (暫定解なし; 予算 2·root_iters + 1000 反復、後戻り 10 回) →
shift-and-propagate → locks / clique / vbounds (サブ MIP; 暫定解なしか gap 1% 超) → local search / repair (暫定解なし)
→ RENS (more nodes) → RINS。

- サブ MIP: ノード上限 500、時間は残りの 7%・最大 6 秒 (`solve_submip_problem`, heuristics.rs:1147-1200; 既定値 solver.rs:778-779)。
  再帰的にフルの根 (カット 5 ラウンド + 丸め・ダイビング) を回す。サブ MIP の中では RENS/RINS/locks などは走らない。
- Feasibility Jump は根の LP の前に 1 回 (時間 min(5%, 5 秒)) (solver.rs:1373-1395)。threads=1 では直列。
- 再スタート (`maybe_restart`, solver.rs:4006-4136): 根で失敗したヒューリスティクスは `failed_heurs` に記録され、再スタート後の
  根では**飛ばされる** (`skip_heurs`, solver.rs:1474, 4090)。
- ダイビングは 1 ステップ 1 列固定 + 伝播 + LP 解き直し (heuristics.rs:459-663)。30n20b8 では深さ 1031 で予算 6031 反復を使い切る
  (ステップあたり 6 反復しか使っていないのに列が 8000 本ある)。

木の中 (solver.rs:2101-2240):

- 毎ノード simple rounding + ZI round。
- LNS (RINS / ALNS = RINS・local branching・proximity・mutation・crossover・DINS) は 100 ノードおき、かつ
  `heur_iters - heur_iters_root < budget` (`budget = 総 LP 反復/20 + 10000`)。**`heur_iters_root` は既定で 0**
  (`ENOMOTO_MIP_HEUR_BUDGET_TREE` で根の分を除く) ので、根のサブ MIP の反復が木の予算を最初から食い潰す
  (solver.rs:1581-1587, 2107-2122)。さらに 1 回のサブ MIP の反復 (RINS は 1000 ノード) をそのまま予算に積むので、
  予算の残りが小さいと 1 回で使い切る。
- randomized rounding も同じ予算 (solver.rs:2162)。
- ダイビング: 深さ % 10 == 3, 7 のノードで、`dive_iters < 0.05·(succ+1)/(calls+1)·node_iters + 1000` のとき 1 回
  (solver.rs:2203-2235)。成功しないと割当は node_iters の 2.5% に縮む。木のダイビングは後戻りなし。
- RINS は LP 解と暫定解の一致率が 50% 未満なら何もしない (heuristics.rs:1570)。h80x6320d では一致率が約 50% で境界。

## 3. 根本原因 (証拠と推定を分けて)

### 根のカット (下界が低い・ノード数が多い)

**C-A. 1 ラウンドに採るカットが少なすぎる (証拠)**。`select_cuts_hybrid` (separation.rs:2159) の「質が最良の 50% 未満は
捨てる」と平行度 0.3 の組み合わせで、p0201 は候補 130-480 本から 1-4 本、assign1-5-8 は 25 ラウンドで 40 本しか採らない。
HiGHS はラウンドごとにプールの候補から平行度 (0.1) とスコアの相対閾値 (最良の一定割合。ただし残りが少なければ上位半分まで
採る `HighsCutPool::separate` の `lower` の規則) で絞るので、1 ラウンドに数十〜数百本入る。こちらの HiGHS 流選択
(`select_cuts_highs`, separation.rs:75) にも「少なすぎれば上位半分」の規則はあるが既定では使われない。

**C-B. 効いていないカットを毎ラウンド即座に外す (証拠)**。`remove_inactive_cuts` が毎ラウンド (`ENOMOTO_T_MIP_CUT_REMOVE_EVERY=1`)
走り、論理変数が基底のカットを全部外す (separation.rs:405-407)。ログでは LP の行数が毎ラウンド増減し (timtab1: 276 → 268 →
277 → 278 → 286 → 283)、外したカットが次のラウンドで再び候補に出て選ばれる循環が起きている。HiGHS は年齢 (`mip_lp_age_limit`
= 10 ラウンド) で外す。C-A と合わさって「毎ラウンド数本足して数本外す」の遅い収束になる。

**C-C. 25 ラウンド・停滞 10 の上限 (証拠)**。7 問が 25 ラウンドの上限で止まり、最後まで下界が伸びている (1.1 節)。
HiGHS は停滞 (LP 解の移動方向の進み具合) で判定し、ラウンド数の上限は事実上ない。以前の計測 (`mip_opt_eval_20261011.md`) で
ラウンド 50 が +3.3% 悪かったのは、C-A/C-B のままだと 1 ラウンドの効率が低く時間だけ増えるためと考えられる (推定)。

**C-D. clique カットが無効、implied bound カットが無い (graph20 は証拠、neos-1456979 は推定)**。graph20-20-1rand は 2 値列だけの
グラフ問題で、根の下界が -37 から 1 本も動かない (HiGHS -21)。HiGHS の根で `Confl. 37` が立っており、clique table と implied
bound による分離が効いている。こちらは `ENOMOTO_MIP_CLIQUE_CUTS` が既定オフ (10teams の退化の副作用で外した経緯が
separation.rs:1675-1678 にある)。neos-1456979 は非零 2 のカットばかり採って下界が動かない (1.3 節)。

**C-E. カットプールの非零上限に当たると黙って捨てる (証拠)**。`add_to_pool` (separation.rs:438-441) の上限 max(20·nnz, 200000)
に timtab1 (17987 本)、qnet1、neos-911970、assign1-5-8、p0201 で到達。以後の根の候補はプールに入らず、木の中でも使えない。
プールが根の全候補 (選ばれなかったものも) を持つ設計 (separation.rs:209-214) のため、質の低い候補で先に埋まる。

**C-F. 面の頂点の分離で候補が 3 倍になる (証拠、副作用は推定)**。`int2` で LP を 2 回 clone して解き直し、候補が 3 倍
(neos-1456979: 421 → 1210)。候補の質の相対閾値 (C-A) と組み合わさると、面の頂点の密なカットが「最良」を押し上げて現在の
頂点の良いカットを落とす可能性がある。

**C-G. CMIR の δ 候補に 1 (と、整数台の行のスケール) が無い (推定)**。cuts.rs:266-291 は境界の内側にある整数列の係数と
その 1/2, 1/4, 1/8 だけ。HiGHS は加えて δ=1 を試し、最良 δ の 1/2, 1/4, 1/8 を再評価する。timtab1 のような大きな係数の
行で弱い CMIR になりうる。

### 主ヒューリスティクス

**H-A. 木の中の LNS の予算が根のサブ MIP で食い潰される (証拠)**。1.4 節の表のとおり、`heur_iters` に根のサブ MIP の反復
(h80x6320d 65k、assign1-5-8 58k、graph20 56k) が積まれ、予算 (総反復/20 + 10000) を最初から超えるので RINS/ALNS/ランダム丸めが
60 秒間一度も走らない。`ENOMOTO_MIP_LNS_TIME=1` で h80x6320d 6860 → 6598、timtab1 1074786 → 996666 (mutation・crossover が
効いたログあり)。以前 `HEUR_BUDGET_TREE` を「40 問で 2 問減った」として不採用にしているが (solver.rs:1581-1584 のコメント)、
それは「解ける問題の時間」の指標で、暫定解の質 (このベンチで負けている指標) を見ていない。

**H-B. 1 回のサブ MIP の反復をそのまま共通予算に積む (証拠)**。timtab1 の HEUR_BUDGET_TREE では RINS (1000 ノード) 1 回で
31k 反復を使い、予算 56k のうち残りを local branching / proximity の 2 回で使い切って以後 40 秒間 LNS なし。HiGHS は
`heuristic_effort` (LP 反復の 5%) を**経過に応じて増える予算**として、サブ MIP ごとの反復も `mip_heuristic_effort` で
抑えている。

**H-C. 根のヒューリスティクスが固定順・固定予算で時間を使い切る (証拠)**。30n20b8: 根 40.6 秒のうち RENS 14.9 秒
(サブ MIP 1601 列 500 ノードで解なし)、root reduced cost 3.8 秒、diving 3 種 6.7 秒、local search 3.0 秒、feasibility pump 2.5 秒、
repair 1.5 秒、LP face 1.4 秒 — すべて失敗。h80x6320d: 根 13 秒 (サブ MIP 4 本が時間上限 6 秒・500 ノードで止まる)。
サブ MIP の上限 (500 ノード、残りの 7%・最大 6 秒、solver.rs:778-779) は大きな問題では短すぎ、小さな問題では無駄。
HiGHS は RENS/RINS のサブ MIP に `mip_heuristic_run_*` の予算を渡し、根の LP 反復に比例したノード上限を使う。

**H-D. ダイビングが 1 列ずつ固定するため予算を深さで使い切る (証拠)**。30n20b8 の fractional diving は深さ 1031 で 6031 反復の
予算切れ (1 ステップ平均 6 反復)。列 8000 本・分数列 1000 本超の問題では、1 ステップで複数列 (例: 分数列の 1/10 か、
伝播で固まる列) を固定しないと届かない。SCIP も `maxdivedepth` に加えて LP を解く頻度 (`lpsolvefreq`) と
「LP 解き直しなしで続けて丸める」規則を持つ。

**H-E. 再スタート後に失敗したヒューリスティクスを飛ばす (証拠、効果は推定)**。`skip_heurs = skip_heurs | failed_heurs`
(solver.rs:4090)。暫定解のない問題で再スタートすると (固定が増えて問題が締まった後なのに) RENS・pump・diving・locks などを
全部飛ばす。30n20b8/neos-1456979 は今回の 60 秒では再スタートに至らなかったので直接の原因ではないが、16 スレッドの
ベンチで neos-1456979 が「解なし」(threads=1 では 266) だったことの一因になりうる (並列では根の FJ・サブ MIP が別スレッドで
走り、`join_submips` の受け取り順によって結果が変わる: `mip_survey` の P4)。

**H-F. RINS の一致率 50% の下限 (証拠)**。heuristics.rs:1570。h80x6320d では一致率が約 50% (6441/12640 自由) で、
境界上。HiGHS は一致率が低ければ固定率の目標 (`fixingRate`) まで伝播で固定を足す (`heuristicNeighbourhood`)。
`mip_opt_eval` の B7 (固定で広げる) は不採用だったが、「一致で足りない分を伝播で固定する」形は試されていない。

**H-G. サブ MIP の中の根が弱い (推定)**。サブ MIP の根のカットは 5 ラウンド (separation.rs:148) で、RENS/RINS/locks は走らない。
HiGHS のサブ MIP (`solveSubMip`) は通常の MIP ソルバを呼び、根のカット・ヒューリスティクスをフルに使う (再帰は深さで制限)。
h80x6320d の根 RENS サブ MIP (6441 列) が 295 ノード・6 秒で 6860 しか出せないのに、HiGHS のサブ MIP は 6470 → 6382 を出す
差の一部はここにあると考えられる。

## 4. 改善の提案 (優先順)

各項目は 1 日程度で実装でき、既定の挙動の変更は環境変数で戻せる形にすることを勧める。検証は (a) 本報告の 16 問 60 秒 threads=1
(根の下界・最初の解・最終目的値・ノード数)、(b) 既存の 40 問 90 秒 SGM (`scripts/mip_bench/bench.py`、解けた問題が減らないこと)、
の 2 本立て。(b) だけで判定すると暫定解の質の改善が見えないので、**(a) に「60 秒時点の主ギャップの和」などの指標を足す**ことを
先に勧める (検証指標の追加は P0)。

### P0. 評価指標の追加 (半日)

`scripts/mip_bench/bench.py` の結果に、時間上限で止まった問題の「60 秒時点の目的値の参照値からの相対差」と「根の下界」
「最初の解の時刻」を出す。本報告の `summarize.py` 相当 (verbose ログの `MIP: cut loop done` / `new incumbent` の解析)。
SGM だけでは H-A のように「解けた問題数は減るが暫定解は良くなる」変更を正しく評価できない。

### P1. 木の中のヒューリスティクス予算を時間ベース・増分方式に (H-A, H-B) — 1 日

変更 (solver.rs:2107-2122, 2162):

```
// 既定を LNS_TIME 相当に: 経過時間の frac (2% + 8%·成功率) を LNS の時間予算にし、根の分は数えない
lns_ok = lns_secs < frac * elapsed
// ランダム丸め・ダイビングの反復予算も根の分を引く: heur_iters_root = heur_iters を根の終わりで必ず設定
// 1 回のサブ MIP のノード上限を予算の残りに合わせる: node_limit = clamp(残り予算反復 / avg_node_iters, 50, 500)
```

期待: h80x6320d 6860 → 6598 (計測済み)、timtab1 1074786 → 996666 (計測済み)、assign1-5-8・supportcase26・graph20 でも
木の中で RINS/ALNS が動く。リスク: 解ける問題でサブ MIP の時間が増える (以前の計測で 15 → 14 問)。対策は成功率連動の
frac (既にある) と、P0 の指標で主ギャップの改善と SGM の両方を見ること。検証: 16 問 60 秒の暫定解、40 問 SGM。

### P2. 根のカット選択で「本数が少なすぎれば相対閾値を緩める」+ 年齢で外す + ラウンド上限を停滞のみに (C-A, C-B, C-C) — 1 日

変更:

- `select_cuts_hybrid` (separation.rs:2159-2170): `min_score` で切った結果が `max_cuts/4` 本未満なら、閾値を候補のスコアの
  上位 `max(max_cuts/4, len/2)` 番目まで下げる (HiGHS の `lower` 規則)。平行度は 0.3 のまま。
- 根のループの除去を年齢制に: `ENOMOTO_T_MIP_ROOT_CUT_AGE` の既定を 0 → 5〜10 (separation.rs:399-407 は既に実装済み。
  `age_cuts` + `remove_cuts_older_than`)。
- `ENOMOTO_T_MIP_CUT_ROUNDS` の既定 25 → 100、停滞判定は `ENOMOTO_MIP_CUT_STALL_ONE` 相当 (全く進まないラウンドも 1 回分)
  に変え、時間の上限 (10%) は残す。

期待: 25 ラウンドで打ち切られる 7 問の根の下界が上がる (p0201 7341 → HiGHS 7399 の方向、timtab1 485k → 569k の方向)、
qnet1・neos-860300・binkar10_1・p0201 のノード数減。1.5 節の根だけの実験結果 (EXP_CUTS) を参照。リスク: 根の時間が増える
(10% の上限で抑える)、LP が密になって木のノード LP が重くなる (年齢制で抑える)。検証: 根だけの下界と時間 (node_limit=1)、
40 問 SGM。

### P3. clique カットを既定で有効にし、implied bound カットを足す (C-D) — 1 日

- `ENOMOTO_MIP_CLIQUE_CUTS` を既定オンにする (separation.rs:1679)。10teams の退化の副作用は、clique カットを「候補に入れるが
  P2 の選択で他と同じ基準で競わせる」「1 ラウンド最大 20 本」に制限して再評価する。
- implied bound cuts: プロービングの含意 (`p.conflict_edges`、mod.rs:844 `probe`) と変数上下限 (`VarBounds`) から
  `x_j <= u_j·y` / `x_j >= l_j·(1-y)` 型の違反カットを分離する小さな分離器を `separate` に足す (HiGHS の
  `separateImpliedBounds` の形。候補は LP 値で違反するものだけ、1 ラウンド 50 本まで)。

期待: graph20-20-1rand の根の下界 -37 → -21 方向 (HiGHS は clique/implied bound で到達)、neos-1456979 154 → 171 方向 (推定)、
bppc4-08・assign1-5-8 の 2 値問題でも効く可能性。リスク: 10teams などで根の LP 解が変わって探索経路が変わる (過去の経緯)。
検証: graph20 と neos-1456979 の根の下界、40 問 SGM。

### P4. カットプールの上限で「質の低いものから捨てる」 (C-E) — 半日

`add_to_pool` (separation.rs:437-463): 上限に当たったら拒否せず、プール内の「LP で効いた回数 0 かつ古い」カットを捨てて
入れる (HiGHS の `agelim` 相当。候補を入れるとき `age=0`、`pool_cut_round` で使われたら `age` をリセット、
根の各ラウンドで `age += 1`)。あわせて上限を `40·nnz` に。timtab1・qnet1・neos-911970・assign1-5-8・p0201 が根で上限に当たる。
期待: 木の中のプールからのカット (`pool_cut_round`) が効く。リスク: 低。検証: `MIP: cut loop done: ... pool N cuts (M nonzeros)` が
上限に張り付かないこと、40 問 SGM。

### P5. 根のヒューリスティクスを時間予算つきの順序にし、サブ MIP の予算を問題の大きさに合わせる (H-C, H-G) — 1 日

- 根のヒューリスティクス全体に時間予算 (例: 残り時間の 15%、暫定解が見つかるまでは 30%) を置き、失敗が続いたサブ MIP 系
  (root reduced cost / LP face / locks / clique / vbounds / repair) は暫定解があればスキップする。
- サブ MIP (heuristics.rs:1147-1200) のノード上限を 500 固定から `clamp(5000·root_iters/サブ問題の反復, 200, 5000)` のように
  問題の大きさで決め、時間上限は `残り時間の 7%` のままで最大 6 秒を外す (HiGHS は時間よりノード・反復で制限)。
- サブ MIP の根のカットを 5 ラウンド固定から「停滞まで (上限 20)」にし、サブ MIP の根でも RINS (暫定解を持ち込む) を 1 回
  許す (再帰深さ 1 まで)。

期待: 30n20b8 の根 40 秒 → 20 秒以下で木に入る、h80x6320d の RENS が 6860 より良い解を出す (HiGHS は 6470)。リスク: 中
(サブ MIP が長くなる問題が出る)。検証: 根の所要時間と根終了時の暫定解 (1.2 節の表)。

### P6. ダイビングで 1 ステップに複数列を固定する (H-D) — 半日

`dive` (heuristics.rs:499-603): 分数列の数が多いとき (例: 100 本超)、スコア順に上位 `max(1, nfrac/20)` 本を続けて
固定・伝播してから LP を解く (SCIP の `lpsolvefreq`/「伝播だけで進める」の簡略形)。後戻りは最後の塊を解いて戻す。
期待: 30n20b8 (深さ 1031 で予算切れ) で解に届く可能性、大きな問題の根の diving 時間 (3 種で 6.7 秒) の短縮。リスク: 低
(予算内の挙動の変更のみ)。検証: `dive ran out of budget at depth ...` のログ、30n20b8/neos-1456979 の最初の解。

### P7. 再スタート後も暫定解がなければヒューリスティクスを飛ばさない (H-E) — 1 時間

solver.rs:4090: `skip_heurs: if self.incumbent.is_none() { 0 } else { self.params.skip_heurs | self.failed_heurs }`。
さらに RENS (no-incumbent 版) は `noinc_rens_done` を再スタートで持ち越さない。期待: 暫定解のない問題の再スタート後に
解を探す。リスク: 根の時間が増える (暫定解なしの問題だけ)。検証: 30n20b8/neos-1456979 の 16 スレッド 60 秒。

### P8. RINS の一致率不足を伝播で補う (H-F) — 半日

heuristics.rs:1547-1582: 一致率が 50% 未満でも、一致した列を伝播しながら固定した後、固定率が目標 (0.6 → 成功/失敗で調整、
RENS の `rens_succ`/`rens_infeas` と同じ仕組み) に届くまで、暫定解との差が小さい列を暫定解の値に固定して足す。
期待: h80x6320d・supportcase26 で根と木の RINS が動く。リスク: 低。検証: `RINS: none` のログが減ること。

### P9. CMIR の δ 候補に 1 と「最良 δ の分割」を足す (C-G) — 半日

cuts.rs:266-291: `deltas` に 1.0 を加え、最良 δ の決定後に δ/2, δ/4, δ/8 を再評価 (全候補の分割は現状のまま)。
期待: timtab1 など係数の大きい行で CMIR が強くなる (推定)。リスク: 低。検証: 根の下界 (node_limit=1)。

### P10 (参考、中期). 分離の並列化と tableau 行の密なカットの抑制

30n20b8 のように tableau 行のカットが平均 1000-1900 非零になる問題では 1 ラウンド 1.2 秒。候補生成は行ごとに独立なので
rayon で並列化できる (C3)。HiGHS の fill の予算 (`ENOMOTO_MIP_TAB_HIGHS_RULES`) は 30n20b8 の下界を下げた経緯があるので、
P2 でラウンド数が増えた後に再評価する。

## 5. 実験の再現

```
# 本報告の計測 (scratchpad に置いたスクリプト。再現するなら同等のものを scripts/mip_bench に足す)
ENOMOTO_MIP_LOG=1 で m.solve(..., time_limit=60, threads=1, log=True) を 1 問 1 プロセスで実行し stderr を保存
HiGHS: highspy 1.15.1、output_flag=True、log_file、threads=1、time_limit=60
根だけ: node_limit=1
設定: ENOMOTO_MIP_LNS_TIME=1 / ENOMOTO_MIP_HEUR_BUDGET_TREE=1 / ENOMOTO_MIP_RENS_DIVE=1 /
      ENOMOTO_T_CUTSEL_MIN_REL=0 / ENOMOTO_T_MIP_ROOT_CUT_AGE=10 /
      ENOMOTO_T_MIP_CUT_ROUNDS=100 ENOMOTO_T_MIP_CUT_STALL=30 ENOMOTO_MIP_CUT_STALL_ONE=1 ENOMOTO_T_MIP_CUT_TIME_FRAC=0.3 /
      ENOMOTO_MIP_CUTSEL_HIGHS=1 / ENOMOTO_MIP_CLIQUE_CUTS=1
```
