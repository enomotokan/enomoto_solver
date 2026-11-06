# 担当 A: 暫定解を改善するサブ MIP (RENS・RINS) の打ち切り値の渡し方と止め方を HiGHS に寄せる (2026-11-06)

bench に使う環境変数: (計測の後で書く)

- 機械: 4 CPU (`nproc` = 4)。ソルバーは同時に 1 つ (`A/queue.sh` の待ち行列)。ENOMOTO は 4 スレッド (`ENOMOTO_MIP_THREADS=4`)、
  `ENOMOTO_MIP_LOG=1`、種は `ENOMOTO_T_MIP_SEED`。HiGHS は highspy 1.15.1 (ソースは v1.15.1 のタグを読んだ)。
- ビルド: ブランチ ccr-1beed81b-ex0uhq の 5db4dd0f + この記録の変更。既定の回 (`def`) の最初の 2 問 (physiciansched6-2・cod105 の 1 回目) は
  変更前の wheel (既定の動きは同じ)。
- 台本: `A/run1.py` (1 回、`S1/run1.py` の写し)、`A/queue.sh`・`A/addq.py`・`A/cfgs.txt` (待ち行列と設定の名前)、`A/summ.py` (集計)。
  結果は `A/res.jsonl`、ログは `A/logs/<設定>_<問題>_s<種>.log` (gzip)。
- 担当 B (`analysis/submip_impr_20261106/B.md`) は近傍の多様化とサブ MIP の中の対称性。環境変数は `ENOMOTO_MIP_SUBA_*` に分けた。

## 1. HiGHS と ENOMOTO のサブ MIP の規則 (事実、ソースから)

HiGHS は `highs/mip/HighsPrimalHeuristics.cpp` (`RENS`・`RINS`・`rootReducedCost`・`solveSubMip`・`determineTargetFixingRate`)、
`HighsMipSolverData.cpp` (`evaluateRootNode`・`moreHeuristicsAllowed`・`checkLimits`)、`HighsMipSolver.cpp` (`runHeuristics`・停滞の数え方)。
ENOMOTO は `src/mip/heuristics.rs` (`rens`・`rens_old`・`dive_neighbourhood`・`rins`・`rins_highs`・`solve_submip_problem`・`target_fix_rate`・`alns`)、
`src/mip/solver.rs` (根のヒューリスティクスの並び・木の大近傍探索の予算・停滞と成果連動の打ち切り・`try_incumbent`)。ENOMOTO の既定は
`ENOMOTO_MIP_SUBMIP_HIGHS=-s` (`p c g t i h f`) を含む今の既定。

| 項目 | HiGHS | ENOMOTO (既定) |
|---|---|---|
| 根で呼ぶもの | `evaluateRootNode` の最後に 1 回だけ: 被約費用のサブ MIP (`rootReducedCost`) → RENS (根の LP 解 `rootlpsol`、**暫定解があっても RENS**)。暫定解がなければ Feasibility Pump。**根で RINS は呼ばない**。再スタートの後の根でもう 1 回 | 根のヒューリスティクスの並び: …→ 被約費用 → LP face → RENS → (暫定解がなければ RENS (LP before cuts)) → 局所探索など → RENS (more nodes) → **RINS** (暫定解か参照解があれば)。並列のサブ MIP (最大 3 本) |
| 木で呼ぶもの | ダイブ (`runHeuristics`) ごと、ヒューリスティクスの努力量 (`mip_heuristic_effort` 0.05: ヒューリスティクスの LP 反復 < 全 LP 反復の 5% (+始めは 1 万)) の中で、暫定解がなければ randomized rounding + RENS、あれば RINS | 大近傍探索 (ALNS の 6 本の腕: RINS・local branching・proximity・mutation・crossover・DINS) を 50 ノードおき、時間の予算 (経過の 2〜10%、改善の率で変える) の中で |
| RENS の近傍 | LP 解へのダイブ: LP で整数値の列を、残りの自由な列の 1 割ずつ固定して LP を解き直す。整数値の列が尽きたら分数の列を `getFixVal` (根の LP 解からの変化 ≥ 0.4 ならその向き、でなければ費用の悪い向き) で丸める (変化の和 0.5 まで)。刈られたら戻る (最大 10 回)。**目標の固定率で止める** | 暫定解あり: `rens_old` = LP で整数値の列を**全部**固定、分数の列は切り下げ・切り上げの範囲 (整数値の列が 5 割未満なら呼ばない)。暫定解なし: ダイビング式 (`dive_neighbourhood`)、**目標 0.8**、1 回 |
| RINS の近傍 | ダイブ: LP 解と暫定解が一致する列を、残りの自由な列の 1 割ずつ固定して LP を解き直す。**目標の固定率で止める**。一致する列が尽きて固定率が 0.25 未満なら分数の列を `getFixVal` で足す | `rins_highs`: 一致する列を**全部**伝播しながら固定 (physiciansched6-2 で 0.88)。0.25 未満なら `getFixVal` で足す。0.1 未満ならサブ MIP にしない。木の中も同じ (根で足した後の LP 解を `getFixVal` の基準に使う) |
| 固定率の目標 | `determineTargetFixingRate`: 0.6 から。実行不能だった固定率の平均 × 0.9 を上の端、解を見つけた固定率の平均の 0.9〜1.1 倍を含む範囲で乱数。統計は RENS・RINS・被約費用で共通 | `target_fix_rate` は同じ式だが、使うのは `ENOMOTO_MIP_FIX_CAP`・ダイビング式 (`RENS_DIVE`・`RINS_DIVE`) だけで、既定の RENS・RINS は使わない |
| 空振りのやり直し | RENS・RINS とも、サブ MIP が**根 (1 ノード以下) で実行不能**なら、目標を「達した固定率 × 0.5」にしてダイブの深さの半分まで戻ってやり直す (深さ ≤ 1・ヒューリスティクスの LP 反復の上限でやめる) | 暫定解のない根の RENS: 既定 1 回 (やり直さない)。RENS はノード上限まで本当に進んだら上限 4 倍の続き (`RENS (more nodes)`、差 > 0.5%)。RINS のやり直しはない (試作 `S1_RINSWIDE` は一致する列の固定を乱数で外してやり直す) |
| サブ MIP の大きさの下限 | 固定率 < 0.1 ならサブ MIP にせず深さ優先の探索 10 回。被約費用は固定率 < 0.3 ならやらない | RINS は 0.1 未満でやらない。RENS は LP で整数値の列が 5 割未満でやらない |
| ノードの上限 | **葉 500・ノード 200 + 外側のノード数 / 20** (外側が並列なら /(スレッド数/4))。根では外側のノード数 ≒ 0 なので **200 ノード** (被約費用は 200 + ノード数/20 も同じ) | **500 ノード** (RENS (more nodes) は 2000)。サブ MIP の中の入れ子の RENS は 50 |
| 停滞 | `mip_max_stall_nodes` = 12: 待ち行列から取ったノードの推定値 ≥ `upper_limit` が 12 回続けば止める。**`upper_limit` は `objective_bound` (外側の上限) から始まるので、解がなくても数える** | `ENOMOTO_T_MIP_SUBMIP_STALL` = 12: 同じ数え方だが、**サブ MIP に暫定解があるときだけ** (打ち切り値より悪い暫定解でも数え始める) |
| 成果連動の打ち切り | **ない** | 外側の根のサブ MIP: 最後に暫定解が良くなったノード k から max(k, 50) ノードで止める (`submip_outcome_stop`)。**打ち切り値より悪い解でも k が決まる** |
| 打ち切り値 | `objective_bound` = 外側の `upper_limit` → サブ MIP の `upper_limit` の初期値。**サブ MIP はそれより悪い解を暫定解にしない** (`addIncumbent` は `upper_limit` 未満だけ) | `MipParams::cutoff` = 外側の刈り込みの上限 (刈り込みにだけ使う)。外側の根のサブ MIP は外側の上限を読み直す (`SubmipCtl`)。**打ち切り値より悪い解も暫定解になる** (`try_incumbent`。試作 `HO1_SUBCUT` で止める) |
| ギャップ | 外側からのサブ MIP: 相対 0、絶対 = feastol × max(外側のギャップ, 1000) | 親と同じ (`SUBMIP_HIGHS` の `g`) |
| 時間の上限 | 外側の残り時間 | 根: 残り時間 (`t`)。木: min(残り × 0.07, 6 秒) と大近傍探索の予算 (2 × 残りの予算、下限 0.3 秒) |
| サブ MIP の中 | ヒューリスティクスの努力量 0.8、対称性の検出なし、強分岐の信頼 0 (`mip_pscost_minreliable` = 0)。サブ MIP の中で暫定解が良くなった後の RENS・RINS はダイブだけ | 深さ 2 まで RENS (`RENS_MAX_DEPTH=2`)、入れ子は 50 ノード・残りの 15% (最大 3 秒)・4 回まで半分でやり直し。強分岐はする (`s` を外した既定) |

### 違いのうち、暫定解の改善に効きそうなもの (推測)

1. **打ち切り値**: ENOMOTO のサブ MIP は外側より悪い解を暫定解にし、それで成果連動・停滞の打ち切りが早く効く (G1 の 2.2)。HiGHS では起きない。
2. **成果連動の打ち切り**: HiGHS にない。ENOMOTO は悪い解を見つけた直後に止まる (neos-933966 の根の RENS: 101 ノード目に 22342 (外側 1393)、201 ノードで止まる)。
3. **固定率**: HiGHS は RENS・RINS とも目標 (初期 0.6、統計で動く) で固定を止めるが、ENOMOTO は全部固定する (RINS 0.88 で空振り。physiciansched6-2)。
4. **根で呼ぶ RENS**: HiGHS は暫定解があっても根で RENS (LP 解へのダイブ) を呼び、RINS は呼ばない。ENOMOTO は暫定解があれば根の RENS は全部固定の形。
5. ノードの上限は HiGHS の方が小さい (根で 200) ので、HiGHS の強みはノード数ではなく、近傍の広さ (固定率) とサブ MIP の中の解き方 (推測)。

(以下、計測の後で書く)
