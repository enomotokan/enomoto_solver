# 幅が許容誤差以下の列の固定 (2026-10-22)

ブランチ `inv/tinywidth-20261022` (develop 1182675 から)。背景は analysis/equiv2_bug_20261022.md (プロービングが伝播の誤差の分だけ
緩めた幅 1e-9 の連続列 `[0, 1e-9]` と、許容誤差の範囲で冗長な行 `x + y <= 1 - 1e-9` を係数の強化が縮めて最適解を切った)。
develop の出口の防御 (53d41e1) に加えて、入口で幅の小さい列そのものを消す。ログ・台本は `benchmarks/tinywidth_20261022/`。

## まとめ

- **入れた処理** (`src/mip/mod.rs` の `fix_tiny_width_columns`、既定で使う。`ENOMOTO_MIP_NO_TINY_WIDTH_FIX` で以前どおり):
  幅 `up - lo` が許容誤差 (既定 1e-6 = MIP の実行可能性の許容誤差 `domain::FEASTOL`、`ENOMOTO_T_MIP_TINY_WIDTH_TOL`) 以下の列
  (許容誤差の内の交差 `lo > up` も) を 1 点に固定する。呼ぶ場所は 4 つ:
  1. MIP の前処理の入口 (`presolve_mip_depth`。再スタートの問題・同値の代入のかけ直し・サブ MIP を含むすべての前処理)
  2. LP の前処理 (`run_extended_mip`) の後、係数の強化の前 (前処理の伝播・行シングルトンが作った幅の小さい列)
  3. プロービングが締めた境界を問題に書き戻すところ (`probe` の最後)
  4. 再スタートで大域的な境界を新しい問題に引き継ぐところ (`solver.rs` の `maybe_restart`)
- **固定する点**: 区間に整数 (0 を含む) があればその整数 (伝播が緩める前の値)、なければ下限 (HiGHS と同じ)。
  整数列は丸め済みなので、交差か幅の小さい区間が 1 つの整数を囲むときだけ当たる (その整数に固定)。
  目的の係数の側の端は**使わない** (§3 の失敗)。
- **行の扱い**: 列の値は固定する点から最大 `s_j` (遠い方の端までの距離) 動く。
  - 不等式の行は失う余裕 `Σ|a_ij| s_j` + 丸めの余裕 `1e-9 (1 + |b|)` だけ右辺を緩める (元の問題で厳密に実行可能な点は 1 つも切らない)。
    `Σ|a_ij| s_j` が許容誤差 (1e-6) を越える列は固定しない (係数の大きな行を持つ列は固定しない)。
  - 等式の行は緩められないので、`Σ|a_ij| s_j <= 1e-9 (1 + |b|)` (前処理の許容誤差) の列だけ固定する。
- **ログ** (`ENOMOTO_MIP_LOG=1`): `MIP: presolve: fixed N columns of width within tolerance (A before, B after the reductions)`、
  `MIP: probing: fixed N columns of width within tolerance`、`MIP: restart: fixed N columns of width within tolerance`。
- **確かめたこと** (§4): 53d41e1 の防御を外しても (`ENOMOTO_MIP_COEF_TIGHTEN_TINY=1`) 固定だけで 2 例とも正しい値になる。
  `cargo test --release --lib` 312 件 (310 + 2) 通る。乱数の MIP × HiGHS で既定・EQUIV2 とも 0/300、幅の小さい列を混ぜた生成
  (fuzz_mixtiny) の食い違いは以前と同じ 1 件 (以前からある 2e-5 相対の差) だけ。55 問 (10 秒) で固定が働いたのは beavma (4 列) と
  uccase12 (131 列) で、どちらもプロービングの書き戻し。前処理後の大きさは変わらず、HiGHS の既知の値との食い違い 0。

## 1. HiGHS 1.15.1 の前処理 (事実、ソースを読んだ)

- `HPresolve::checkColBounds` (HPresolve.cpp 2225 行): `boundDiff = col_upper - col_lower` が `primal_feastol` 以下で、
  かつ `boundDiff <= small_matrix_value` (1e-9) か `getMaxAbsColVal(col) * boundDiff <= primal_feastol` なら固定列。
  `boundDiff < -primal_feastol` なら実行不能。`changeColUpper`・`changeColLower` (境界を変えるたび) と `colPresolve` から呼ばれる。
- `primal_feastol` は MIP では `mip_feasibility_tolerance` (既定 1e-6)、LP では `primal_feasibility_tolerance` (1e-7)
  (HPresolve.cpp 98-101 行)。
- 固定する点: `colPresolve` の固定列は `removeFixedCol(col)` で **下限** (`col_lower`)。行の右辺は固定値 × 係数だけ動かす
  (行を緩める処理はない。行の破れは許容誤差 1e-6 の内として扱う)。
- 行シングルトン (3295 行) では幅が許容誤差の内になったとき「締めなかった側の境界、両方締めたなら中点」に寄せる。
- 整数列の境界は `floor(newUpper + primal_feastol)` などで丸める (幅 < 1 の整数列は丸めで交差か 1 点になる)。

こちらとの違い: HiGHS は幅 1e-9 以下なら係数を見ずに固定し、整数の行の右辺の丸めも 1e-6 の許容誤差で行う。こちらの前処理は
整数の行の正規化 (`normalize_integer_rows` の `floor(b + 1e-9)`)・行の実行不能の判定が 1e-9 の大きさなので、固定でずれた右辺を
厳密に扱う。そのため行を緩める (不等式) / 破れを 1e-9 に限る (等式) 手当てを足した。

## 2. LP の前処理 (presolve.rs) の穴 (事実と推測)

- presolve.rs (`run_extended`・`run_extended_mip`) は交差 (`lo > up`) は `fix_crossed_bounds` で 1 点にするが、幅が正で小さい列は
  そのまま残す (事実)。
- 今回の誤り (許容誤差の範囲で冗長な行を厳密な行に縮める) は MIP 側の係数の強化 (`tighten_coefficients`、presolve_mip の中) にあり、
  LP の前処理にはこの手順はない (事実)。MIP では LP の前処理の出力に対して固定を呼ぶ (上の 2) ので、そこで作られた幅の小さい列も
  係数の強化の前に消える。純粋な LP の前処理には手を入れていない (同じ形の誤りは見つけていない。推測: LP では解の値が許容誤差の内で
  動くだけで、最適値を大きく変える経路はない)。

## 3. 最初の版の失敗 (事実、cef7e95 → 0f61380 で直した)

最初の版 (cef7e95) は「整数があればそれ、なければ目的の係数の側の端」に固定し、行は緩めず各行の破れの和を 1e-6 までに限った。
乱数の突き合わせで悪くなった:

- fuzz_mixtiny (幅の小さい連続列を `[x0, x0 + w]` で混ぜたもの): 1 → **11/300**。10 件はこちらが「実行不能」と答えた (HiGHS は最適)。
  seed 68: 列 [1.42477614, 1.42477614 + 1e-8] (費用 -9.77) を上端に固定すると、連続列だけの等式の連鎖
  (`-5 x3 - 4 x10 = b`、`7 x10 + 5 x3 - 6 x5 - 4 x13 = b'`、x5・x13 も固定) が 1e-7 の大きさで破れ、前処理 (許容誤差 1e-9) が
  実行不能と判定した。元の問題で値を取れる端 (下端 = x0) でなく、緩めた側の端に固定したのが原因。
- fuzz_tiny (幅 1e-9 の列とクリーク型の行): 14 → **111/300**、目的の側をやめても 45。`x + y <= 1 - 2e-9` (y は `[0, 1e-9]`) で y を 0 に
  固定すると整数だけの行 `x <= 1 - 2e-9` になり、整数の行の正規化が x = 1 を切る (HiGHS は 2e-9 の破れを許容誤差の内として x = 1)。
  厳密には x = 1 は元の問題で実行不能なので誤りではないが、HiGHS との食い違いが増えた。行を `|a| s` だけ緩めても
  `0.999999998 + 1e-9 = 0.9999999989999999` が `floor(b + 1e-9)` の境目のわずかに下で 0 に丸められた (seed 21)。
  → 緩めに丸めの余裕 `1e-9 (1 + |b|)` を足して 10/300 (以前の動作 14/300) に。

## 4. 確かめたこと (0f61380、事実)

`run_all.txt`・`verify1.txt` に全部の出力。

### 4.1 防御を外して固定だけで正しい値になるか (`ENOMOTO_MIP_COEF_TIGHTEN_TINY=1`)

| 例 | 固定あり | 固定なし (`ENOMOTO_MIP_NO_TINY_WIDTH_FIX=1`) | 正しい値 |
|---|---|---|---|
| fuzz_tiny seed 41 | **-10.9** (最適) | -9.81 (誤った最適) | -10.9 |
| uccase12 の再スタート後の問題を EQUIV2 で直接解く (1 スレッド・60 秒、根の LP) | **11468.554** (+38.80 = 11507.355) | 12004.79 (= 12043.6) | 11507.356 |
| uccase12 を EQUIV2+RESTART で解く (4 スレッド・45 秒、再スタート後に 1350 組を使った回) | 根の LP 11507.357、下界 11507.357、時間切れ (暫定解 11507.904) | (develop の調査で 11507.9037 の誤った最適) | 11507.405 |

- 直接解いた回の前処理のログ: `fixed 131 columns of width within tolerance (131 before, 0 after the reductions)`、組 1106 個を使う。
  60 秒では暫定解なし (時間切れ。最初の版では 11469.1 の暫定解があった。下界は正しい)。
- 全経路の回のログ (`uc_rep/rep0/log.txt`): 元の問題のプロービングの書き戻しで 131 列 (元の問題で締めた 131 本が全部)、
  再スタートの引き継ぎでは 0 列 (固定済み)、再スタート後のプロービングで 199 列、組の代入のかけ直しの入口で 16 列を固定。
  (最初の版 cef7e95 では 67 / 53 / 263 / 11 列。目的の側の端に固定した列が再スタートまで幅を残していた。)
- 回帰テスト `presolve_fixes_tiny_width_columns`・`tiny_range_columns_keep_exactly_feasible_optimum`・
  `fix_tiny_width_columns_fixes_within_tolerance` は `ENOMOTO_MIP_COEF_TIGHTEN_TINY=1 ENOMOTO_MIP_NO_TINY_WIDTH_FIX=1` で落ち、
  `ENOMOTO_MIP_COEF_TIGHTEN_TINY=1` だけでは通る (固定だけで守れる)。

### 4.2 テスト

- `cargo test --release --lib`: 312 件通る (310 + `fix_tiny_width_columns_fixes_within_tolerance`・`presolve_fixes_tiny_width_columns`)。

### 4.3 乱数の MIP × HiGHS 1.15.1 (各 300 問、`ENOMOTO_MIP_REL_GAP=1e-9`)

| 生成 | 既定 | 防御なし (`COEF_TIGHTEN_TINY`) | EQUIV2+RESTART | 以前の動作 (`NO_TINY_WIDTH_FIX`) |
|---|---|---|---|---|
| scripts/mip_fuzz_vs_highs.py | **0** | — | **0** | — |
| fuzz_mixtiny.py (幅 1e-9〜5e-7 の連続列を混ぜる) | 1 | 1 | 1 | 1 |
| fuzz_tiny.py (幅 1e-9 の列とクリーク型の行) | 10 | 13 | 10 | 14 |

- fuzz_mixtiny の 1 件は seed 204 (HiGHS 0.2016604、こちら 0.2016557、相対 2e-5 だけこちらが良い)。4 設定とも同じで、固定とは関係ない
  以前からの差 (推測: 許容誤差の内の破れの扱いの差。追っていない)。
- fuzz_tiny の分類 (`fuzz_tiny_cls.py`): 既定は「こちらが悪い / HiGHS の点が 2e-9 破る」4 (以前 8)、「こちらが良い」6 (以前と同じ)。
  **HiGHS の整数解を丸めて連続列を 0 にした厳密に実行可能な点より悪い値を最適と返したものは 0** (どの設定でも)。
  残りは右辺 `1 - 2e-9` の行を 2e-9 破る点を HiGHS が許容誤差の内として採る差で、厳密にはこちらが正しい。

### 4.4 55 問 (40 問 + highsonly16)、4 スレッド・10 秒、既定 / `ENOMOTO_MIP_NO_TINY_WIDTH_FIX=1` を 1 回ずつ (`check55.jsonl`、`cmp55.txt`)

- 固定が働いた問題: **beavma** (`probing: fixed 4 columns`) と **uccase12** (`probing: fixed 131 columns`) だけ。どちらも
  プロービングの書き戻し (上の 3)。前処理の入口・LP の前処理の後・再スタートで固定したものはなかった (beavma は再スタートが 1 回あるが、
  幅の小さい列はプロービングで固定済み)。
- 前処理後の大きさ (最初の `rows a -> b` の行) が変わった問題: 0。2 回目以降の前処理の行が違う問題が 10 問 (app1-1・drayage-25-23・
  gt2・h80x6320d・khb05250・neos-2978193-inde・neos-860300・nw04・p0201・qnet1) あるが、固定のログはなく、LP による固定・再スタートの
  時刻の違い (4 スレッド・時間で決まる) によるもの (推測。app1-1 は LP による固定の後の前処理が 3535 行 / 3511 行)。
- 落ちたもの 0。HiGHS の既知の解の値 (`equiv2_bug_20261022/cmp_known.py`、110 行すべて既知の値あり) より上の下界・既定のギャップを
  越える「最適」: **0**。
- 状態・目的値が変わった問題 (時間切れの暫定解の差が主): app1-1 (既定 最適 -3 / 止めた方 時間切れ)、beasleyC3・bppc4-08・
  drayage-25-23・eil33-2・gen-ip002・h80x6320d・mc11・mzzv42z・neos-860300・neos-911970・nw04・qiu・ran14x18-disj-8・roll3000・
  rout・supportcase26・timtab1・uccase12。固定が働いていない問題がほとんどなので、10 秒・4 スレッドのばらつき (推測)。
  正式な計測は親が別に立てる。

## 5. 残り・推測

- uccase12 の直接の求解 (60 秒) で最初の版にあった暫定解が最終版では出なかった。1 回ずつなのでばらつきか、行を緩めた影響かは分からない。
- 行を緩めるのは固定した列を含む不等式の行だけで、緩めの最大は 1e-6 + 1e-9 (1 + |b|)。解の検査 (`is_feasible(x, 1e-6)`) の内だが、
  緩めた行が 1e-6 近く破れる解が出ると、元の問題の空間での検査に掛かりうる (起きた例は見ていない)。
- 整数の行の丸めの許容誤差 (1e-9) は HiGHS (1e-6) より厳しい。fuzz_tiny の残りの食い違いはこれによる (以前から)。変えていない。
- 53d41e1 の出口の防御は残してある (2 重の守り)。固定だけで今回の 2 例は守れるので、外すかどうかは別に計測して決める。
