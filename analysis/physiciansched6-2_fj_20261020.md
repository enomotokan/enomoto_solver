# physiciansched6-2: Feasibility Jump を通すのに要る縮約の特定と試作 (2026-10-20、担当 physician2)

機械は 4 CPU (`nproc` = 4、専用コンテナ)。基準はブランチ `inv/physician2-20261020` の起点 `c3ed13f` (統合ブランチ + 前の調査 #10 の
試作)、試作のコミットは `e2526b0` (と、その後のメモのコミット)。試作はすべて環境変数で有効にし、既定は変えていない。
**計測はすべて 4 スレッド** (ENOMOTO は `ENOMOTO_MIP_THREADS=4`・`m.solve(None, False, T, None, None, 4, log=True)`、HiGHS 1.15.1 は
`threads=4`)。時間上限は求解が 60 秒、FJ の確かめは 2〜4 秒 (FJ は根の前に回すので短くてよい)。
スクリプト・小さいログは `analysis/physiciansched6-2_fj_20261020/`:

| ファイル | 内容 |
|---|---|
| `trial.sh <mps> [環境変数...]` | 前処理後に FJ を種を変えて 20 回回し (`ENOMOTO_MIP_DEBUG_FJ_TRIALS=20`)、解を出した回数と各回の最小の違反行数を出す。この調査の主な物差し |
| `run_cfg.sh <問題> <秒> <名前> [環境変数...]` | 求解 1 回 (ログは `logs/`)。`cmp_ps62_60.txt`・`cmp_ps33_60.txt`・`cmp_side_60.txt` はその要約 |
| `highs_then_fj.sh` | ENOMOTO の前処理後の問題に HiGHS の前処理 (`presolve_rule_off` で規則を選ぶ) をかけてから FJ |
| `cmp_pipeline.sh <名前> [環境変数...]` | ENOMOTO の前処理後と、それに HiGHS の基本の縮約をかけた後を、同じ ENOMOTO の前処理を通してから列の名前で突き合わせる |
| `rowset2.py`・`colcmp.py` | 2 つの問題の行 (同じ・倍率だけ違う・片方だけ) と列 (費用・境界) の違い |
| `hybrid2.py`・`bndswap.py`・`intscale.py`・`shufrows.py`・`droprows.py` | 片方の問題の行・列の境界を他方に移した混合の問題、整数行の拡大、行の並べ替え、行の削除 (どの違いが FJ に効くかを確かめる) |
| `mpsq.py`・`rowq.py`・`rename.py`・`mpsmodel.py` | MPS の列・行の表示、ENOMOTO の書き出し (`C{j}`) の列名を元の名前に戻す、小さな MPS の読み書き |
| `highs_domspr_log.patch` | HiGHS 1.15.1 に当てる記録用のパッチ (`HIGHS_DOM_LOG`: 対の支配 `HDOM`・双対の支配 `HDDC`、`HIGHS_SPARSIFY_LOG`: 疎化 `HSPR`)。`highs-bin` を作って使う |
| `screen_new.sh <秒>` | 40 問で新しい試作が何をするかの数え上げ (`screen_new_40.txt`) |

設定の略号: `min` = `ENOMOTO_MIP_INEQ_SINGLETON` + `ENOMOTO_MIP_CLIQUE_EXTEND` + `ENOMOTO_MIP_DUAL_UB` + `ENOMOTO_MIP_CS_OBJ_PIVOT`
+ `ENOMOTO_MIP_FJ_LP_REPAIR` (この調査で特定した最小の組)、`minpc` = min + `MERGE_REPRESOLVE`・`PROBE_EQUIV`・`PROBE_CLIQUE`、
`full` = 前の調査の試作 7 つ (`CLIQUE_EXTEND`・`MERGE_REPRESOLVE`・`PROBE_EQUIV`・`INEQ_SINGLETON`・`PROBE_CLIQUE`・`SPARSIFY`・`DOMCOL`)、
`pc` = full から `SPARSIFY`・`DOMCOL` を除いたもの。

## 1. 結論

事実 (4 スレッドで確かめた):

1. **ENOMOTO 単独で FJ が解を出すようになった**。最小の組 `min` (5 つ) で、FJ を種を変えて 20 回回すと 18 回解を出す
   (基準・前の調査の `full` は 0 回)。求解 60 秒 2 回: 基準は 2 回とも解なし、`min` は 2 回とも **51831 (10.1・10.2 秒)**、
   `minpc` は 51329 (9.9 秒) / 52031 (10.0 秒)。最適 49324 との差は 4〜6%、HiGHS は 12.8 秒で最適を示して終わる。
2. FJ を止めていた主因は、前の調査が見ていた 0-1 列の行 (2 変数の行・疎化・支配される 0-1 列) ではなく、**目的の費用を運ぶ
   連続列の形** だった。HiGHS の前処理後との差を 1 つずつ入れ替えて確かめた (§2.4〜2.6):
   - (a) 目的の費用を 1 本の連続列 `C62277` (費用 1) が持ち、それを定める 37 列の等式 `0.000999·C62277 = Σ(罰則の列) + Σ(0-1 列) + …`
     が残る。HiGHS は Free col substitution でこの列を消し、費用を等式の列に配る。この等式を HiGHS 側の問題に戻すだけで
     FJ は 20 回中 19 回 → 1 回。ENOMOTO の列シングルトンの代入は、係数 0.000999 が行の最大係数 1 の 1% 未満なので
     (`SUBSTITUTION_PIVOT_RATIO` = 1e-2) 代入しない → `ENOMOTO_MIP_CS_OBJ_PIVOT`。
   - (b) 罰則の連続列 416 本 (`100Σx + 600z − w ≤ 600` の形の行だけに出る費用つきの w) の上限が ENOMOTO では ∞、HiGHS では
     100〜600 (w がほかの列の最悪の値でも要る量)。上限だけを入れ替えると ENOMOTO 側 0 → 19 回、HiGHS 側 19 → 0 回
     → `ENOMOTO_MIP_DUAL_UB`。
   - (c) 上の 2 つの後も、連続列 2 本だけの 2 行 (`0.5a − b ≤ 43.75`・`b − a ≤ −925`、a は費用 1・下限 1759.57、b は下限
     834.57。両方を満たすのは a ≥ 1762.5) が 1 行破れて残る。1 変数ずつ動かす FJ は 2 行の間を往復して止まり、前処理の
     境界の伝播も等比的にしか近づかない (下限が 1762.5 に届かない)。HiGHS は Probing か Enumeration を有効にしたときだけ
     この 2 列を固定する (理由は確かめていない)。FJ が解なしで終わったとき整数列を固定して連続列を LP で解き直す
     (`ENOMOTO_MIP_FJ_LP_REPAIR`) と通る。
3. 最小の組 (§2.7): `INEQ_SINGLETON` (前の調査) + `CLIQUE_EXTEND` (前の調査) + `DUAL_UB` + `CS_OBJ_PIVOT` + `FJ_LP_REPAIR`。
   どれか 1 つを外すと 20 回中 0〜1 回。前の調査の `MERGE_REPRESOLVE`・`PROBE_EQUIV`・`PROBE_CLIQUE`・`SPARSIFY`・`DOMCOL` は
   要らない (外しても 16〜20 回)。前処理後は 15862 × 9547 と大きいまま (HiGHS 8861 × 4608) で通る。
4. 課題 1 (支配される列): HiGHS の前処理の報告の「Dominated col」は対の支配 (`dominatedColumns`) ではなく、双対の境界による
   `detectDominatedCol` だった。ENOMOTO の前処理後 (full) に HiGHS をかけると、対の支配は 5 列 (すべて予測の境界、Gamrath ら
   2015 の定理 3)、双対の境界は 20 列 (連続列 7・整数列 13)。前の調査の「33〜72 列」は pc / Sparsify を切った後の数で、多くが
   連続列の双対の固定 (pc では 112 列中 104 列が連続列)。対の支配の予測の境界が固定するのは、係数も費用も同じ 0-1 列が 3 本
   ずつの組 13 組の各 2 本 (§2.2)。両方の試作 (`ENOMOTO_MIP_DOMCOL_PRED`・`ENOMOTO_MIP_DUALPROP`) を作ったが FJ には効かない。
5. 課題 2 (疎化): HiGHS の選び方は「短い等式から・フィルイン 1 列まで・倍率は整数に限らない・同じ長さの行も書き換える」
   (§2.3)。同じ選び方を `ENOMOTO_MIP_SPARSIFY=highs` にした。最小の組には疎化は要らない。

推測:

- (b) が FJ に効く仕組みは確かめていない。上限のない罰則の列は、FJ の初めの点 (下限) から行を満たすために大きく動けるので、
  0-1 列を動かしたときの行の違反がすぐ罰則の列で埋まり、重みが整数列の行に移らないのではと見ている。
- (a) も同じく、目的が 1 本の列に集まると FJ の目的の重みがその列にしか働かず、37 列の等式が 0-1 列を動かすたびに破れる
  (等式を直すには C62277 が動く必要がある) ことが障害と見ている。
- 課題 4 の根のカット (ENOMOTO 48587〜48690、HiGHS 49324) は今回調べていない。

## 2. 調べたこと

### 2.1 物差し: FJ を種を変えて 20 回 (`ENOMOTO_MIP_DEBUG_FJ_TRIALS`)

FJ (`fj2_search`) は種を固定すると決定的で、前の調査の `fjcheck.sh` の `ENOMOTO_MIP_FJ_SEED` はどこでも読まれていなかった
(3 回とも同じ結果になっていたのはそのため)。しかも 1 回の成否は行・列の並びだけで変わる (ENOMOTO の前処理後 `full` を HiGHS で
前処理なしで書き直すだけで、最小の違反が 17 → 10 行)。そこで根の前に FJ を種を変えて 20 回 (各回は手間の上限まで) 回し、
解を出した回数と最小の違反行数の分布を見る診断 `ENOMOTO_MIP_DEBUG_FJ_TRIALS=N` を足し (`src/mip/solver.rs`)、`trial.sh` で
比べた。行の並べ替え (`shufrows.py`) では結果は変わらない (0/20 と 17〜18/20 のまま) ので、構造の違いを見ている。

### 2.2 HiGHS の支配される列 (`highs_domspr_log.patch`、`HIGHS_DOM_LOG=1`)

HiGHS 1.15.1 には 2 つの「支配」がある。

- `dominatedColumns` (`HPresolve.cpp` 1045 行): 列の対 j ≻ k (費用・全行の係数で j の方が行を破りにくい) を、j の
  `bestRowPlus` / `bestRowMinus` (j を増やすと行を締める向きの最も短い行) の列 k について調べる。固定は (i) j の境界が行から
  導ける (`isUpperImplied`)、(ii) j と k が共通のクリークにある、(iii) 予測の境界 (Gamrath ら 2015 の定理 3。j が上限にあるときに
  k が要る最大の値など。`computeColBounds`) のどれか。連続列・一般整数列も対象 (整数列が連続列を支配するのは除く)。
- `detectDominatedCol` (4514 行): 連続列 (整数列は除く) の費用と、境界が無限か行から厳密に導けるときだけ作る「双対の行」から
  行の双対の境界を伝播し (`updateRowDualImpliedBounds`)、列の被約費用の符号が決まれば固定する (強い: > 0、弱い: ≥ 0)。
  前処理の報告の「Dominated col」はこちら。

ENOMOTO の前処理後に HiGHS の前処理をかけて数えた:

| 入力 | 前処理の設定 | 対の支配 (`HDOM`) | 双対の支配 (`HDDC`) |
|---|---|---|---|
| full (9522 × 4731) | 既定 | 5 (すべて予測の境界) | 20 (強い・連続列 7、弱い・整数列 13) |
| full | Sparsify を切る | 5 | 20 |
| full | 規則 6〜19 を全部切る | 33 (予測の境界 32、導ける境界 1) | 13 (弱い・整数列) |
| pc (9868 × 4977) | 既定 | 17 (すべて予測の境界) | 112 (強い・連続列 7、弱い・連続列 97、弱い・整数列 8) |
| pc | Sparsify を切る | 17 | 38 |

予測の境界で固定する 0-1 列は、係数も費用 (0) も同じ列が 3 本ずつの組 (例 `C8850`・`C8863`・`C8878`、行は
`100Σ + 600z − w ≤ 600`・`Σ + 3z ≤ 3`・`y ≤ Σ`・`Σ ≤ 3y`) の各 2 本 (13 組 26 本) と、目的の鎖の連続列 6 本。3 本の和が
1 を超えても得をしない (y ≤ Σ のほかは Σ が小さいほど楽) ので 1 本で足りる。ENOMOTO の DOMCOL (0-1 列どうし、(i) と (ii)
だけ) はこれを拾わない。

試作:
- `ENOMOTO_MIP_DOMCOL_PRED` (`dominated_binary_fixings_with`): DOMCOL に定理 3 (vi) に当たる (c) を足す。y_j ≻ y_k、c(y_k) ≥ 0
  のとき、y_j = 1・y_k = 0 で、y_k を下げると破れうる行 (y_k の係数が上限側で負・下限側で正) がほかの列の最悪の値でも満たされる
  なら y_k = 0 (支配の補題で「y_j = 1 か y_k = 0 の最適解がある」ため)。候補は HiGHS の bestRowPlus の行 (別の上限
  `ENOMOTO_T_MIP_DOMCOL_PRED_MAX_PAIRS`)。**physiciansched6-2 では 0 本** (ENOMOTO が DOMCOL を回す時点では 3 本の列がまだ
  同じ形になっていない)。40 問では fiber だけ (2 回目の DOMCOL で 8 → 14 本)。
- `ENOMOTO_MIP_DUALPROP` (`src/presolve/dualpropagate.rs` `mip_dual_fixings`、`run_extended_mip` から): 既存の `dualpropagate`
  (MIP では切ってある) の MIP 版。双対の行は連続列からだけ作り、境界は無限か 1 本の行から厳密に導けるとき無限とみなす。
  被約費用が正なら固定、0 以上なら双対の行を作らなかった列 (整数列など) だけ固定 (弱い支配)。physiciansched6-2 では
  連続列 97 本を固定、整数列 0 本で、FJ には効かない。40 問では rout (253)・binkar10_1 (162)・fiber (44、整数列 44)。

失敗: `DOMCOL_PRED` の最初の版は許容誤差を `1e-9 (1 + max(|上限|, |下限|).min(1e9))` にしていて、片側が無限の行で許容誤差が約 1
になり、正しくない固定をしていた (physiciansched6-2 で 26 本と数えたのはこの誤り)。単体テスト
`dominated_binary_fixings_keep_optimum` (6 列の 0-1 問題を総当たり) に予測の境界を足して見つかった。直した後は上の数。

### 2.3 HiGHS の疎化の選び方 (`HPresolve::sparsify`、8301 行、`HIGHS_SPARSIFY_LOG=1`)

ENOMOTO の `sparsify_rows` との違い:

| | HiGHS | ENOMOTO (`ENOMOTO_MIP_SPARSIFY`) |
|---|---|---|
| 等式の順 | 短い順 (`equations` は (長さ, 行) の集合の写し) | 長い順 |
| 対象の行 | 等式の最も出現の少ない列を含む行 + (その列が一般整数でなければ) 2 番目に少ない列を含み最も少ない列を含まない行 | 等式の最も出現の少ない列を含み、等式の列をすべて含み、等式より長い行 |
| フィルイン | 1 列まで (等式が連続列だけで対象に整数列があれば 0、一般整数列は不可) | なし |
| 倍率 | 消える非零の最も多いもの、同数なら \|s\| の小さい方、\|s\| ≤ 1e3。整数に限らない | 整数列だけの行では整数だけ |
| 書き換えた行・使った等式 | 書き換えた等式もその後に等式として使う | 互いに使わない |
| いつ | Probing の後に 1 回 | 行をまとめた直後 |

ENOMOTO の full の前処理後に HiGHS をかけると 90 行を書き換えた。ほとんどが 3〜4 列の等式 `Σ = 1` を同じ列の `Σ ≤ 1`・
`2x + y + z ≤ 2` から引いて冗長にするもの (ENOMOTO は「等式より長い行」に限るので書き換えない)。同じ選び方を
`sparsify_rows_highs` (`ENOMOTO_MIP_SPARSIFY=highs`) に、前処理の最後 (プロービング・同値の代入の後) に 1 回だけかける形を
`ENOMOTO_MIP_SPARSIFY_LATE` (`=old` で今の選び方) にした。FJ への効果は §2.7。

### 2.4 FJ を通す HiGHS の規則の二分探索 (ENOMOTO の full の前処理後に HiGHS の前処理、`highs_then_fj.sh`)

HiGHS の後の問題を ENOMOTO (pc) で解く。FJ は 20 回。

| HiGHS の前処理 | HiGHS の後 | FJ (LP の修復なし) | FJ (`FJ_LP_REPAIR`) |
|---|---|---|---|
| (HiGHS をかけない) | 9522 × 4731 | 0/20 (違反 14〜25) | 0/20 |
| 前処理なし (書き直すだけ) | 8070 × 4730 | 0/20 (9〜14) | 0/20 |
| 規則 6〜19 を全部切る (基本の縮約) | 8007 × 4654 | 0/20 (すべて 1) | **20/20** |
| 基本 + Sparsify | 7917 × 4654 | 0/20 (1〜2) | |
| 基本 + 6〜13・17〜19 のどれか 1 つ | 8004〜8007 × 4651〜4654 | 0/20 (1。Aggregator だけ 3〜7) | |
| 基本 + Probing (15) か Enumeration (16) | 7970 × 4654 | 18/20 | |
| Sparsify だけ切る | 7956 × 4644 | 19/20 | |
| 既定 | 7903 × 4644 | 16/20 | |

基本の縮約の後に残る 1 行は §1 の (c) の連続列 2 本の行。Probing・Enumeration を入れたときは、この 2 列が固定される
(ENOMOTO の前処理を通した後の差はこの 2 列と 2 行だけ)。Probing・Enumeration が消す 37 行 (等式と同じ列の `Σ ≤ 1`) は
ENOMOTO の前処理を通すと消えるので関係ない (行を消した問題でも 0/20)。

### 2.5 ENOMOTO 単独と HiGHS の基本の縮約の後の違い (`cmp_pipeline.sh`)

ENOMOTO の前処理後 (full + DOMCOL_PRED + DUALPROP) と、それに HiGHS の基本の縮約をかけた後を、同じ ENOMOTO の前処理を通して
列の名前で突き合わせた (`rowset2.py`): 行は 7883 本が同じ、81 本が倍率だけ違い (整数列だけの等式で ENOMOTO は係数 0.5 と 1、
HiGHS は 2 倍して 2 と 1)、ENOMOTO だけに 10 本・HiGHS だけに 3 本 (目的の連続列の部分)。FJ は 0/20 対 19/20。
混合の問題 (HiGHS 側を土台に ENOMOTO 側の要素を戻す、`hybrid2.py`) で切り分けた:

| HiGHS 側に戻したもの | FJ |
|---|---|
| (何も戻さない) | 19/20 |
| 倍率だけ違う 81 行 | 18/20 |
| 目的の鎖の 9 行 (`C110727 ≥ 837.5 + Σ …`・`y_k ≤ y_822` など) | 18/20 |
| 目的を定める等式 1 行 (`0.000999·C62277 = …`、37 列。HiGHS では C62277 を代入で消して費用を配ってある) | **1/20** (違反 6〜8) |
| ENOMOTO だけの 10 行と HiGHS だけの 3 行を全部入れ替え | 0/20 (5〜10) |

### 2.6 目的の列を代入した後の違い

列シングルトンの代入の比を下げる (`ENOMOTO_T_CS_SUBSTITUTION_PIVOT_RATIO=1e-4`) と、FJ の違反は 15〜20 → 8〜14 行。同じ突き合わせを
やり直すと、行の違いは倍率と目的の鎖だけになり、列の違いは罰則の連続列 416 本の上限だけ (ENOMOTO [0, ∞)、HiGHS [0, 100〜600])。
上限を入れ替えた問題 (`bndswap.py`): ENOMOTO 側に HiGHS の上限 → 19/20、HiGHS 側を ∞ に → 0/20 (8〜12)。

### 2.7 元の問題で最小の組を探す (`trial.sh`、時間上限 4 秒)

| 設定 | 前処理後 | FJ |
|---|---|---|
| full | 9522 × 4731 | 0/20 (15〜20) |
| full + FJ_LP_REPAIR | | 0/20 |
| full + DUAL_UB | | 0/20 (6〜9) |
| full + 代入の比 1e-4 | 9515 × 4724 | 0/20 (8〜14) |
| full + DUAL_UB + 比 1e-4 | | 0/20 (1〜2) |
| full + DUAL_UB + 比 1e-4 + FJ_LP_REPAIR | | **19/20** |
| 上から CLIQUE_EXTEND を外す | | 8/20 |
| 上から INEQ_SINGLETON を外す | 9526 × 4735 | 0/20 (8〜13) |
| 上から MERGE_REPRESOLVE・PROBE_CLIQUE・SPARSIFY をそれぞれ外す | | 19/20 |
| 上から PROBE_EQUIV を外す | 11934 × 5833 | 16/20 |
| 上から DOMCOL を外す | 9687 × 4872 | 20/20 |
| DUAL_UB + 比 1e-4 + FJ_LP_REPAIR だけ | 15870 × 9555 | 0/20 (23〜46) |
| **min** (INEQ_SINGLETON + CLIQUE_EXTEND + DUAL_UB + CS_OBJ_PIVOT + FJ_LP_REPAIR) | 15862 × 9547 | **18/20** |
| min − INEQ_SINGLETON | | 0/20 (8〜13) |
| min − CLIQUE_EXTEND | | 1/20 |
| min − DUAL_UB | | 0/20 (10〜14) |
| min − CS_OBJ_PIVOT | 15889 × 9574 | 0/20 (9〜10) |
| min − FJ_LP_REPAIR | | 0/20 (すべて 1〜3) |

INEQ_SINGLETON は目的の費用を持つ `y ≥ Σ(罰則の列)` の形の連続列を消す (前の調査)。CLIQUE_EXTEND が効く理由は確かめていない。

FJ の後の追加 (`trials_extra.txt`): min + `SPARSIFY` (今の選び方) と min + `SPARSIFY=highs` は 20/20 (LP の修復なしで解を出す回がある)、
min + `SPARSIFY_LATE` は 19/20。逆に新しい試作なしで `SPARSIFY=highs` を前の調査の pc に足しても 0/20 (違反 14〜22)。
疎化は (a)(b)(c) の後でだけ効く。

### 2.8 求解 (60 秒・2 回、`cmp_ps62_60.txt`)

| 設定 | 前処理後 | 根 (カット後) | 最初の解 | 60 秒の暫定解 / 下界 |
|---|---|---|---|---|
| 基準 | 15892 × 9577 | 46619.25 | なし / なし | なし |
| min | 15862 × 9547 | 48587.4 | 51831 (10.1 秒) / 51831 (10.2 秒) | 51831 / 48587.4 (77・222 ノード) |
| min + SPARSIFY=highs | 11922 × 5894 | 48587.7 | 51529 (8.8 秒) / 51530 (8.7 秒) | 同じ / 48587.7 |
| minpc | 9839 × 4948 | 48587.4 / 48639.9 | 51329 (9.9 秒) / 52031 (10.0 秒) | 同じ / 48587.4・48690.0 |
| (参考) HiGHS 既定 | 8861 × 4608 | 49324 | 49324 (12.7 秒、根のサブ MIP) | 最適 (12.8 秒) |

FJ が解を出すのは根の LP と並行に回るスレッドで、受け取るのは根の LP・カットの後 (8.7〜10.2 秒)。木の中で暫定解はほとんど
良くならない (51831 → 51831)。

### 2.9 physiciansched3-3 (`cmp_ps33_60.txt`、`trials_extra.txt`)

60 秒 2 回は基準・min とも解なし・根の LP も解けない (前処理後 73397 × 17914 → min 72955 × 17385)。FJ を 5 回: 基準は違反 959〜985 行、
min は 370〜402、minpc (前処理後 43460 × 11761) は 277〜305 で、どれも解なし。同じ構造でも、この問題は (a)(b)(c) だけでは足りない。

## 3. 試作 (既定では使わない)

| 環境変数 | 変えた関数 | 内容 |
|---|---|---|
| `ENOMOTO_MIP_DUAL_UB` | `src/mip/mod.rs` `dual_bound_tightening` (`solve_problem_inner` で前処理の後、クリーク行の拡張の前) | 費用が非負 (非正) の連続列が、増やす (減らす) と楽になる向きの行にしか出ないとき、上限 (下限) を「ほかの列が最悪の値でもその行を満たす値」の最大まで締める。1 回だけ |
| `ENOMOTO_MIP_CS_OBJ_PIVOT` | `src/presolve/colsingleton.rs` `eliminate_singleton_equalities_view` | 整数最適化の前処理で、費用のある連続列の代入はピボットの比の下限を `ENOMOTO_T_MIP_CS_OBJ_PIVOT_RATIO` (1e-6) にする (既定は全列 1e-2) |
| `ENOMOTO_MIP_FJ_LP_REPAIR` | `src/mip/fj.rs` `fj2_search`・`lp_repair` | FJ が解なしで終わったら、違反の行の数が最も少なかった点 (`ENOMOTO_T_MIP_FJ_LP_REPAIR_MAX_VIOL` 行以下、50) の整数列を固定し、連続列だけの LP (`solve_lp`) を元の費用で解く。FJ の各回 (種ごと) に 1 回 |
| `ENOMOTO_MIP_DOMCOL_PRED` | `src/mip/mod.rs` `dominated_binary_fixings_with` | DOMCOL (`ENOMOTO_MIP_DOMCOL` と併用) に予測の境界 (c) を足す (§2.2) |
| `ENOMOTO_MIP_DUALPROP` | `src/presolve/dualpropagate.rs` `mip_dual_fixings`、`src/presolve.rs` `run_extended_mip` | 連続列から作る双対の箱による固定 (§2.2)。`ENOMOTO_DEBUG_DUALPROPAGATE` で数を出す |
| `ENOMOTO_MIP_SPARSIFY=highs` | `src/mip/mod.rs` `sparsify_rows_highs` | HiGHS と同じ選び方の疎化 (§2.3) |
| `ENOMOTO_MIP_SPARSIFY_LATE` (`=old`) | `src/mip/mod.rs` `solve_problem_inner` | 前処理の最後に 1 回疎化し、書き換えたら前処理をかけ直す |
| 診断用 | `src/mip/solver.rs` | `ENOMOTO_MIP_DEBUG_FJ_TRIALS=N`: 根の前に FJ を種を変えて N 回回し `MIP: FJ trials: found k/N` を出す (`ENOMOTO_MIP_FJ_DEBUG=1` で各回の最小の違反) |

単体テスト: `dual_bound_tightening_keeps_optimum` (0-1 列 5 本と罰則の連続列 2 本の問題 500 個で、総当たりの最適解が締めた境界の中に
ある)、`dominated_binary_fixings_keep_optimum` に予測の境界あり・なしの両方 (予測の境界で固定が増えることも確かめる)。
`cargo test --release --lib` 308 件通過 (`e2526b0` の後に `DOMCOL_PRED` の候補の上限を分けた版でも同じ)。

### 3.1 40 問で働く問題 (`screen_new_40.txt`、3 秒)

| 試作 | 働く問題 |
|---|---|
| DUAL_UB | neos-911970 (48 本、3 秒の FJ の成功が 1 → 20 回)、mas74・mas76・flugpl・bppc4-08 (各 1 本) |
| CS_OBJ_PIVOT | 前処理後の大きさが変わる問題はない |
| FJ_LP_REPAIR | pk1 (4 回)、khb05250 (10 回)、supportcase26 (5 回) で修復した解 |
| DUALPROP | rout (連続列 253)、binkar10_1 (連続列 162)、fiber (整数列 44) |
| DOMCOL_PRED | fiber (2 回目の DOMCOL で 8 → 14 本)。air03・nw04・blp-ar98 は DOMCOL と同じ |

### 3.2 副作用 (60 秒・2 回、`cmp_side_60.txt`。`new` = DUAL_UB + CS_OBJ_PIVOT + FJ_LP_REPAIR + DUALPROP)

| 問題 | base (2 回) | new (2 回) |
|---|---|---|
| neos-911970 | 時間切れ 54.76 / 54.89、下界 53.49 / 53.48 | 時間切れ 55.33 / 54.83、下界 53.05 / 53.00 (最初の解 80.57 は 1.1〜1.6 秒 → 0.4 秒) |
| mas74 | 時間切れ 11801.2 / 11923.3、下界 11168.9 / 11153.7 | 時間切れ 11857.4 / 11801.2、下界 11163.5 / 11173.2 |
| mas76 | 最適 13.2・11.2 秒 | 最適 12.7・12.0 秒 |
| flugpl | 最適 0.1・0.1 秒 | 同じ |
| bppc4-08 | 時間切れ 54 / 54、下界 51.59 | 時間切れ 54 / 54、下界 51.59 (最初の解 290 → 94) |
| pk1 | 最適 26.1・21.5 秒 | 最適 19.0・31.1 秒 (最初の解 42 → 51) |
| khb05250 | 最適 0.8・1.1 秒 | 最適 1.2・1.1 秒 |
| supportcase26 | 時間切れ 1785.2 / 1810.7、下界 1437.9 / 1432.3 | 時間切れ 1789.5 / 1776.8、下界 1430.6 / 1437.3 (最初の解 2201.9 → 1891.4) |
| rout | 最適 9.6・7.5 秒 | 最適 7.9・8.9 秒 |
| binkar10_1 | 最適 38.2・25.9 秒 (16324・8039 ノード、根 6706.6) | 最適 21.1・22.2 秒 (4939・6293 ノード、**根 6718.0**) |
| fiber | 最適 1.7・1.5 秒 | 最適 1.6・1.6 秒 |
| fiber (DOMCOL / DOMCOL + PRED) | 最適 1.5・1.4 秒 | 最適 1.5・1.6 秒 |

最適値はどの問題も一致。悪化ははっきりしない (neos-911970 の下界がやや低いのは根のカット後の値 47.03 → 46.85 の差で、
境界を締めるだけの DUAL_UB で LP が弱くなることはないので、回ごとの揺れか DUALPROP と見ている。確かめていない)。

## 4. 既定にする候補か

- `ENOMOTO_MIP_DUAL_UB`: 軽く (1 回の走査)、正しさも総当たりで確かめた。physiciansched6-2 で必須、neos-911970 で最初の解が早くなる。
  **既定の候補** (40 問の全体の計測を親で)。
- `ENOMOTO_MIP_FJ_LP_REPAIR`: FJ が解なしのときだけ LP を 1 回解く。physiciansched6-2 で必須、pk1・khb05250・supportcase26 でも修復で
  解が出る (supportcase26・bppc4-08 では最初の解が良くなる)。LP の時間が FJ の時間の上限に入らないので、大きな問題で FJ の各回ごとに
  LP を解く分の時間は見ていない。**既定の候補** (時間の上限の扱いを足してから)。
- `ENOMOTO_MIP_CS_OBJ_PIVOT`: 40 問では何も変えない。復元の誤差の増幅 (1/比) を許すので、目的値の誤差を問題ごとに見る必要がある。
  候補だが、前処理後の解の研磨 (`polish_solution`) があるので実害は小さいと見ている。
- `ENOMOTO_MIP_DUALPROP`: binkar10_1 で根の下界が上がりノードが半分。physiciansched6-2 には効かない。候補 (優先は低め)。
- `ENOMOTO_MIP_DOMCOL_PRED`・`ENOMOTO_MIP_SPARSIFY=highs`・`ENOMOTO_MIP_SPARSIFY_LATE`: 候補にしない (働く問題が少ない、または
  min の後でだけ効く)。
- 前の調査の `INEQ_SINGLETON`・`CLIQUE_EXTEND` は min に必須なので、既定にするならこの 3 つと組で。

## 5. 確かでない点・うまくいかなかったこと

- (b) の上限・(a) の目的の列が FJ に効く仕組み (§1 の推測)。
- (c) の 2 列を HiGHS が Probing・Enumeration を有効にしたときだけ固定する理由 (どの関数かは追っていない)。ENOMOTO はこれを
  前処理では解かず、FJ の後の LP で直している。
- CLIQUE_EXTEND が min に要る理由 (外すと 1/20)。
- physiciansched3-3 は違反を 959 → 277 行まで減らしたが解は出ない。残る違反の形は見ていない。
- 課題 4 の根のカットの差 (48587〜48690 対 49324) と、HiGHS 流の「根のサブ MIP で当てる」道は調べていない。暫定解 51529〜51831 は
  60 秒でほとんど良くならないので、次はこちら。
- `DOMCOL_PRED` の最初の版の誤り (§2.2)。physiciansched6-2 では直した後 0 本で、HiGHS が固定する 3 本組は ENOMOTO の DOMCOL の
  時点ではまだ同じ形でない。
- 前の調査の `fjcheck.sh` は `ENOMOTO_MIP_FJ_SEED` を渡していたが、この変数はどこでも読まれていない (種は変わっていなかった)。
- HiGHS の計測用ビルドはこのコンテナの `/tmp/HiGHS` (`highs_domspr_log.patch` を当てて `ninja highs-bin`)。
