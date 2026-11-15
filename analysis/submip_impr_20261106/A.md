# 担当 A: 暫定解を改善するサブ MIP (RENS・RINS) の打ち切り値の渡し方と止め方を HiGHS に寄せる (2026-11-06)

bench に使う環境変数: `ENOMOTO_MIP_SUBA_CUT=1` (打ち切り値より悪い解をサブ MIP の暫定解にしない)、`ENOMOTO_MIP_SUBA_STOP=<e s o n>` (止め方)、`ENOMOTO_MIP_SUBA_RATE=<i t n z>` (固定率の目標、`1` = `inz`)、`ENOMOTO_T_MIP_SUBA_NOIMP` (既定 100)、`ENOMOTO_T_MIP_SUBA_RATE_TRIES` (既定 1)。どれも既定では使わない (既定の動きは変えていない)

- 機械: 4 CPU (`nproc` = 4)。ソルバーは同時に 1 つ (`A/queue.sh` の待ち行列)。ENOMOTO は 4 スレッド (`ENOMOTO_MIP_THREADS=4`)、
  `ENOMOTO_MIP_LOG=1`、種は `ENOMOTO_T_MIP_SEED`。HiGHS は highspy 1.15.1 (ソースは v1.15.1 のタグを読んだ)。
- ビルド: **2 つの土台**。(旧) 5db4dd0f + この記録の変更 (種 1 の `def`・`cut`・`ext`・`cuts`・`rate`・`ratew`・`all`・`cutn`。`def` の最初の 2 問は変更前の wheel、既定の動きは同じ)。
  (新) 16:10 に作り直した wheel: 上流の 354bde33 まで + `SUBA_STOP` の `n`。上流で **`ENOMOTO_MIP_S4_RINSRATE` が既定 (外側の根の RINS は固定率 0.5 未満なら飛ばす)** になっていて、
  既定の動きが旧と違う。新の回は `d2` (新の既定、種 1)・`def` の種 2・`cutx`・`cutnx`。**新の回どうし、旧の回どうしで比べる**。
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

## 2. 試作 (どれも環境変数、既定では使わない)

| 名前 | 中身 | 場所 |
|---|---|---|
| `ENOMOTO_MIP_SUBA_CUT=1` | (a) サブ MIP の `try_incumbent` で、打ち切り値 (`MipParams::cutoff`) と外側の今の上限 (`SubmipCtl::best`、外側の根のサブ MIP だけ) の小さい方より悪い解を捨てる (HiGHS の `objective_bound` → `upper_limit`)。捨てた解は暫定解にも成果連動の「見つけたノード」にもならない。捨てた数は終わりのログ (`SUBA_CUT rejected N`) | `solver.rs` の `suba_cut`・`try_incumbent` |
| `ENOMOTO_MIP_SUBA_STOP=e` | (b) 成果連動の打ち切りの「見つけたノード k」を、外側の打ち切り値より良い解のときだけ決める (悪い解は暫定解にはする) | `solver.rs` |
| `ENOMOTO_MIP_SUBA_STOP=s` | (b) 停滞 (12) を、サブ MIP に暫定解がなくても打ち切り値が有限なら数える (HiGHS) | 同上 |
| `ENOMOTO_MIP_SUBA_STOP=o` | (b) 外側の根のサブ MIP の成果連動の打ち切りを使わない (計測はしていない) | 同上 |
| `ENOMOTO_MIP_SUBA_STOP=n` | (b) 外側の根のサブ MIP が打ち切り値より良い解をまだ見つけていなければ `ENOMOTO_T_MIP_SUBA_NOIMP` (100) ノードで止める。`SUBA_CUT`・`e` で成果連動の打ち切りが効かなくなる分の歯止め (cod105 の害を見て足した) | 同上 |
| `ENOMOTO_MIP_SUBA_RATE=<文字>` | (c) 固定率の目標 (`target_fix_rate`、HiGHS の `determineTargetFixingRate`) を使う。`n`: 根の `rens_old` を目標で止める (`force_fix_cap`)。`z`: 暫定解のない RENS のダイブの目標を 0.8 から `target_fix_rate()` に。`i`: 外側の根の RINS を一致する列を乱数の順に目標まで固定 (実行不能なら 0.5 倍で 1 回やり直す)。`t`: 木の RINS も目標まで。`1` = `inz`。`S1_RINSWIDE` と組める (`ratew`) | `heuristics.rs` の `suba_rate`・`rins_highs`・`suba_rins_solve` |

計測の設定の名前 (`A/cfgs.txt`): `cut` = CUT、`ext` = STOP=e、`cuts` = CUT + STOP=s、`rate` = RATE=1、`ratew` = RATE=1 + `S1_RINSWIDE=2`、
`all` = CUT + STOP=e + RATE=1、`cutn` = CUT + RATE=nz、`cutx` = CUT + STOP=n、`cutnx` = CUT + RATE=nz + STOP=n、`d2` = 新の土台の既定。

固定率 (c) は親のセッションの連絡で担当 C (`C.md`、`ENOMOTO_MIP_SUBFIX_*`) に移った。`SUBA_RATE` はここに記録だけ残す。

## 3. 7 問の結果 (4 スレッド、600 秒。physiciansched6-2 は 150 秒、cod105 は 300 秒)

全部の列 (最初の解・暫定解の推移・サブ MIP の本数と秒・捨てた解の数) は `A/summ.md` (`python A/summ.py` の出力)。生は `A/res.jsonl`・`A/logs/`。

### 3.1 事実

最適まで解けた問題は秒、解けなかった問題は時間切れのときの暫定解。「根」は根のヒューリスティクスの後の暫定解。

| 問題 | 旧 def (種 1) | 旧の試作 (種 1) | 新 d2 (種 1) / 新 def (種 2) | 新 cutx (種 1 / 2) | 新 cutnx (種 1 / 2) |
|---|---|---|---|---|---|
| physiciansched6-2 | 74.3 秒 (根 49424) | cut 19.9 / ext 81.0 / rate 25.7 / ratew 25.8 / all 26.0 / cutn 25.3 (rate 系は根 49324) | 68.9 (根 49424) / 17.5 (根の前に Local-MIP が最適) | 88.2 (根 49424) / 16.3 | 30.9 / 25.6 (どちらも根 49324) |
| neos-933966 | 319.5 秒 (根 2389) | cut 134.8 (根 376) / cuts 154.8 (376) / ext 288.1 (2389) / rate 140.4 (390) / all 時間切れ 382 / cutn 時間切れ 358 | 374.8 (根 2389) / 146.8 (根 384) | 時間切れ 376 (根 1382) / 415.3 (根 2371) | 141.6 (根 391) / 時間切れ 320 (根 1335) |
| cod105 | 217.9 秒 | cut・ext・cuts・cutn は**時間切れ** (300 秒、-12) / rate 210.5 | 148.7 / 147.2 | 157.0 / - | 148.9 / 147.4 |
| trento1 | 1.539e7 | cut **5.806e6** / cutn **5.585e6** / cuts 1.528e7 / ext 1.521e7 / rate 1.533e7 | **5.312e6** / 3.568e7 | 1.561e7 / **5.509e6** | 1.541e7 / 1.541e7 |
| nexp-150-20-8-5 | 242 | cut 247 / ext 238 / cuts 240 / rate 248 / cutn 240 | 241 / 237 | 245 / 242 | 242 / 241 |
| fiball | 140 | cut 140 / rate 143 / cutn 141 | 139 / 142 | 140 / - | 141 / 141 |
| thor50dday | 73875 | cut・ext・rate・cutn どれも 73875 | - | - | - |

ログから読んだこと:

- **cod105 の害** (旧): 外側の根の RINS (固定率 0.32、700 列が自由) は 1 ノード約 0.7 秒。既定は打ち切り値 (-12) より悪い -2 を見つけて成果連動の打ち切りで 113 ノード・78 秒で止まる。`cut` は -2 を捨てる (918 個捨てた) ので成果連動が効かず、停滞も推定値が上限に届かないので効かず (`cuts` も同じ)、243 ノード・238 秒の時間切れ。`ext` も 343 ノード・238 秒。新の土台では `S4_RINSRATE` (既定 0.5) でこの RINS が飛ばされ、cutx・cutnx も d2 と同じ 148〜157 秒 (`n` の歯止め自体はこの問題では試せていない)。
- **neos-933966**: 解けるまでの秒は根の後の暫定解でほぼ決まる (根 376〜391 なら 135〜155 秒、1335〜2389 なら 288 秒〜時間切れ)。根の 376〜391 は外側の根の RINS (500 ノード) か、`rate` では目標 0.60 で止めた RENS が見つける。良い根になった回は旧 cut・cuts・rate、新 def 種 2・cutnx 種 1 で、悪い根は旧 def・ext・all・cutn、新 d2・cutx の 2 回・cutnx 種 2。**同じ設定でも種で入れ替わる** (新 def は種 1 が悪く種 2 が良い、cutnx は逆)。
- **physiciansched6-2**: 根の RENS を目標 (0.60) で止める `SUBA_RATE` の `n` を含む設定 (rate・ratew・all・cutn・cutnx) は 6 回とも根で 49324 (最適) を見つけ、25〜31 秒。含まない設定で根が 49424 の回は 69〜88 秒。ただし根の前の Local-MIP が最適を見つけた回は 16〜20 秒 (旧 cut、新 def 種 2・cutx 種 2)。
- **trento1**: 5.3e6〜5.8e6 に下がった 5 回 (旧 cut・cutn、新 d2・cutx 種 2) は、どれも 200〜450 秒の木の大近傍探索の **local branching** が下げた (RENS・RINS ではない)。下がらなかった回は 1.5e7 前後。
- nexp-150-20-8-5・fiball・thor50dday は設定の差がばらつき (±5) の中。thor50dday はどの設定も 73875 で、根の後に暫定解が動かない。

### 3.2 解釈 (推測)

- **(a) 打ち切り値を渡すこと (`SUBA_CUT`) だけでは良くならない**。neos-933966・trento1 の旧 cut の良い結果は、新の d2・def 種 2 も同じくらい良い回があり、種によるばらつきの中。打ち切り値より悪い解を捨てると成果連動の打ち切りが効かなくなり、1 ノードが重い問題 (cod105) では根のサブ MIP が時間を使い切る害が出る。HiGHS は根のサブ MIP を 200 ノードで止めるので、HiGHS に寄せるならノードの上限 (または `n` の歯止め) と組にする必要がある。
- **(b) 成果連動の打ち切りを延ばすこと (`e`)・停滞を数え始めること (`s`) も、7 問では差が見えない**。`s` は打ち切り値より推定値が良いノードが続く問題 (cod105) では効かない。
- **(c) 固定率の目標で根の RENS を止めること (`SUBA_RATE` の `n`) は physiciansched6-2 で毎回効いた** (6/6 回で根で最適)。neos-933966 でも `rate` では RENS が 390 を見つけた。HiGHS が強い理由の 1 つは、根で LP 解に寄せた RENS を目標の固定率 (0.6) で止めて呼ぶことだと思う。ただし RINS も目標で止める (`i`) と、neos-933966 で `all` (0.63 に絞った RINS が 500 ノードで何も見つけない) のように悪くなった。固定率は担当 C に移ったので、C に「根の RENS を目標で止めるのは効く、根の RINS を目標で絞るのは neos-933966 で悪い」と伝える。
- 7 問は 1 回の結果が種で大きく動く (neos-933966 は 135〜600 秒、trento1 は 5.3e6〜3.6e7)。2 回ずつでは、ここに書いた以外の差は言えない。

## 4. 58 問の害 (`A/bench58.py`、4 スレッド・60 秒、新の土台、既定 `def` と `cutnx`、回 0・1 は種 1・2)

事実 (`python A/cmp58.py def cutnx`、`A/bench58.jsonl`): 秒のずらした幾何平均 (解けなければ 60 秒) と解けた数。

| 回 | def | cutnx | 差 |
|---|---|---|---|
| 0 | 16.25 秒・43 問 | 15.94 秒・43 問 | -1.9% |
| 1 | 16.76 秒・41 問 | 16.97 秒・41 問 | +1.2% |

1.5 倍を超えて動いた問題: 10teams (4.8・4.3 → 8.1・8.5、2 回とも遅い)、swath1 (21.2 → 35.1、回 0)、misc07 (7.4 → 11.1、回 0)、
mik-250-20-75-4 (4.2 → 8.8、回 1)、neos-960392 (58.7 → 解けず、回 1) が悪く、drayage-25-23 (40.6 → 5.0、回 0)・neos-911970 (24.2・26.9 → 11.0・15.6、2 回とも)・
physiciansched6-2 (解けず → 29.2、回 1) が良い。

解釈 (推測): 全体ではばらつきの中 (±2%)。2 回とも同じ向きに動いたのは 10teams (悪い) と neos-911970 (良い) だけ。10teams の原因は見ていない。

## 5. cargo test

`cargo test --release --lib` (RUSTFLAGS に `-L/usr/local/lib -lpython3.13`): 349 passed、0 failed、18 ignored。

## 6. まとめと次

- 既定は変えない。`SUBA_CUT`・`SUBA_STOP` は 7 問でも 58 問でも、ばらつきを超える良さを示さなかった。打ち切り値より悪い解を捨てると成果連動の打ち切りが効かなくなり、
  1 ノードが重い根のサブ MIP が時間を使い切る (cod105 の旧の回)。HiGHS に寄せるなら、打ち切り値とノードの上限 (根で 200、または `n` の 100) を必ず組にする。
- 効いたのは、根の RENS を固定率の目標で止めること (`SUBA_RATE` の `n`、physiciansched6-2 で毎回)。固定率は担当 C の領分なので、C に渡す。
  根の RINS を目標で絞る (`i`) と neos-933966 で悪くなった。
- 次にやるなら: (1) 根の RENS を目標で止めることを単独 (`SUBA_RATE=n`) で 58 問・種 3 つ。(2) 10teams が cutnx で 2 倍遅い理由をログで見る。
  (3) 7 問は 1 回のばらつきが大きい (neos-933966 は 135〜600 秒) ので、根の後の暫定解のように種に左右されにくい量で比べる。
