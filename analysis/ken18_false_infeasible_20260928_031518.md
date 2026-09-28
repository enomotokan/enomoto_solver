# Kennington ken-18 の誤った infeasible の調査 (2026-09-28 03:15 UTC)

対象: HEAD `bcd59a2` (ブランチ claude/compassionate-lamport-v8hmvy)、/home/user/venv_new。
問題: /home/user/kenn/ken-18.mps (105,127 行 × 154,699 列、全行が等式、係数は ±1、全列 [0, ub]、ub は 72〜420003)。
HiGHS 1.15.1: optimal、obj = -52217025287.3968、10.8 s。enomoto: **0.25 s で "infeasible"** (誤り)。

計装・試作は別 clone `/home/user/prof_ken18` (+ `/home/user/venv_ken18`) で行い、本体の src は触っていない。

## 1. 結論 (要約)

- **誤判定の段**: 前処理 `presolve::run_extended` のラウンド 0、`rowsingleton::fix_singleton_equalities`
  (`src/presolve/rowsingleton.rs:49`、`value < lb[j] - TOL`、`TOL = 1e-9` 絶対)。単体法には到達していない。
  - 行 2990 (縮約後の A の番号。元の行 3749 = MPS 名 R3750)、列 6404 (C6405)。
  - `value = b'/coeff = 3.505485700047136e5`、`lb[6404] = 3.505485700047277e5`、差 **1.41e-8 (相対 4.0e-14)** > 1e-9。
  - `value` はスケール後座標で HiGHS の最適解 x[C6405] = 36 (36 / d_j = 36 / 1.0269618e-4 = 3.505485700047136e5) と**ビット単位で一致**する。
    つまり実行可能解 (最適解) そのものを「境界の外」として排除している。
- **機序**: Ruiz スケーリング後の座標 (列スケール d_j ≈ 1e-4 なので x' = x/d は 1e5〜1e9) で、
  `propagate_equalities` (eqprop) が行 3749 から `lb[6404] = (b - l_s)/a_ik` を出す際、
  `l_s = finite_sum_inf - contrib_k` の**桁落ち**で `l_s` に 2.66e-10 (= ulp(4.29e6)/3.5) の誤差が入り、
  それが `1/|a_ik| = 53` 倍されて `lb` が真値より 1.41e-8 大きくなる。その後 foldfixed が同じ行の固定列を右辺へ畳み込み、
  rowsingleton が `b'/coeff` を (桁落ちなしで) 計算すると真値が出るので、`lb` と 1.41e-8 食い違い、
  **絶対 1e-9** の許容誤差で実行不能と判定される。
  スケーリングを切る (`ENOMOTO_T_RUIZ_ITERS=0`) と係数 ±1・整数データで演算が厳密になり、症状は出ない。
- **壊れたコミット**: なし (退行ではない)。54de561 (セッション開始時点) のビルドでも同じ infeasible。
  `propagate_equalities` と rowsingleton の絶対許容誤差はこのリポジトリの最初のコミット f3eb480 (2026-09-22) から存在する。
  venv_s1 (a666575)・venv_s2 (474bf5a) も infeasible。ken-18 が既定設定で正しく解けた記録はない。
- **他の問題への影響**: Netlib93 + Kennington 16 + Mittelmann 11 の前処理だけを回した掃引では、誤判定は ken-18 のみ。
  ただし「相対 1e-6 以内の際どい判定」は Kennington の ken-07/11/13 (各 77〜187 件)、Netlib の 20 問ほどで多数発動しており、
  その相対違反はいずれも 3e-16 以下 (単純な丸め) で、ken-18 の 4e-14 (桁落ち + 1/|a_ik| の増幅) だけが突出している。
  同種の桁落ちは「スケール後の上限が大きい列が、他の項が小さい等式行に小さい係数で入る」構造で起きうる
  (pds-*、fome13、dfl001、agg* などはスケール後の上限が 1e9 級で、潜在的に同じ危険がある)。
- **対処法** (§6): (A) 判定側の許容誤差を相対化 (試作で ken-18 が optimal、obj は HiGHS と一致、掃引で他の前処理結果に変化なし)、
  (B) 伝播側で桁落ちを避ける / 誤差分だけ緩める、(C) rowsingleton の HiGHS 流クランプ、(D) 伝播で得た境界は「弱い境界」として扱う、の組み合わせを推奨。

## 2. 再現と切り分け

```
python /tmp/claude-0/sp/run_storm.py /home/user/kenn/ken-18.mps ours
```

| 設定 | 結果 |
|---|---|
| 既定 | infeasible 0.25 s |
| `ENOMOTO_DEBUG_PRESOLVE_INFEAS=1` | `DEBUG_PRESOLVE: infeasible=true` (前処理で確定、単体法に入らない) |
| `ENOMOTO_T_PRESOLVE_FIXPOINT=0` / `_OPPOSITE_PAIR_EQUALITY=0` / `_LARGE_PRESOLVE_MIN_ROWS=0` / `ENOMOTO_INEQ_SINGLETON=0` / `_PRESOLVE_ROUNDS=0,1` / `_EQPROP_ROUNDS=0` / `DISABLE_AGGREGATOR` / `DISABLE_FREEVAR` / `DISABLE_PARALLELCOLS` | すべて infeasible (無関係) |
| `ENOMOTO_T_PROPAGATION_PASSES=1` | **optimal** -52217025287.39681 (9.8 s) |
| `ENOMOTO_T_PROPAGATION_PASSES=0` | optimal (8.7 s) |
| `ENOMOTO_T_RUIZ_ITERS=0` | optimal (5.8 s) |
| `ENOMOTO_T_RUIZ_ITERS=1` | optimal (6.4 s); `=3` は infeasible |
| 54de561 ビルド (venv_ken18 で再ビルド) | infeasible。`PROPAGATION_PASSES=1` で optimal (53 s) |
| venv_s1 (a666575), venv_s2 (474bf5a) | infeasible |

`ENOMOTO_T_EQPROP_ROUNDS=0` でも infeasible なのは、ラウンド 0 の等式伝播が無くても後のラウンドで別の列が同じ機序に当たるため
(パス 2 の伝播が要る点は同じ)。

## 3. 誤判定の場所 (計装の出力)

別 clone に各 infeasible 判定箇所へ eprintln を入れて実行:

```
DBG_INFEAS rowsingleton row=2990 j=6404 coeff=-1.888290021904761e-2 b=-6.619373669328833e3
           value=3.505485700047136e5 lb=3.505485700047277e5 ub=1.051645710014155e6
DBG_INFEAS presolve site 5      (= src/presolve.rs:673 付近、内側ループの rowsingleton の直後)
```

列 6404 (C6405) の境界の履歴 (`DBG_TRACE_COL=6404`):

```
scaling d[6404]=1.0269618e-4   scaled bounds lb=0 ub=4.0897625e9        (元: [0, 420003])
A 行 3749 (R3750, e_a=183.87): -0.0188829 x6404 + 0.0227359 x6405 + 0.0188518 x6406 + 0.974715 x6407 = 0
A 行 3834 (R3835), A 行 104990 (R104991, 325 項, b=23334)
eqprop pass=1 row=104990: ub <- 2.2721390e8 (l_s=0, b=23334, aik=1.027e-4)
eqprop pass=2 row=3749  : lb <- 3.505485700047277e5   (l_s=6.619373669329099e3, b=0, aik=-1.888290e-2)
eqprop pass=2 row=3749  : ub <- 1.0516457e6
foldfixed -> singleton j=6404: folded=-6.619373669328833e3 value=3.505485700047136e5
    fixed_terms=[(6405, 0.0227359, 186007.49845810703), (6406, 0.0188518, 58521.23785042912), (6407, 0.974715, 1320.4890574193275)]
rowsingleton: value 3.505485700047136e5 < lb 3.505485700047277e5 - 1e-9  →  infeasible
```

列 6405/6406/6407 は pass 1 で各自の行 (3878/3877/3871) から lb/ub を得て、pass 2 で同じ行の強制行判定により lb に固定される。
行 3749 は pass 2 で (番号順に) それらの行より先に処理されるので、そのとき 3 列はまだ `[lb, ub]` の区間であり、
`<=` 側の l_s は「他 3 項の下限側寄与」= 固定後の値と同じ数値で計算されている。したがって食い違いは順序の問題ではなく、下の桁落ちによる。

## 4. 機序 (数値で確認)

`propagate_equalities` (`src/presolve/propagate.rs:701`) は列 k を除いた最小活動度を

```
l_s = finite_sum_inf - contrib_k          (contrib_k = a_ik * ub[k]、a_ik < 0 のとき)
```

で計算する。行 3749 では列 6404 が先頭で、pass 1 で得た `ub[6404] = 2.272e8` により

```
contrib_k      = -0.0188829 * 2.2721390e8 = -4290457.3666699715
finite_sum_inf = contrib_k + 4229.1... + 1103.2... + 1287.1... = -4283837.993000642   (ulp = 9.3e-10)
l_s            = finite_sum_inf - contrib_k = 6619.373669329099
真値 (3 項の和) =                             6619.373669328833     差 2.66e-10
lb 候補        = (0 - l_s) / a_ik = 350548.5700047277           真値 350548.5700047136   差 1.41e-8
```

(Python で同じ順序で計算すると上の値がビット単位で再現する。) つまり誤差は
「自分自身の大きな寄与を足してから引く」桁落ち ≈ ulp(|contrib_k|) が `1/|a_ik|` (= 53) 倍されたもの。
伝播自体は数学的には正しく、HiGHS の解もこの境界上にある (x = 36 ⇔ 3.505485700047136e5)。
問題は、後段の判定が**スケール後の量の大きさ (1e5〜1e9) に無関係な絶対 1e-9** を使っていること。

同じ絶対許容誤差で「境界の外」「矛盾」を判定している箇所 (すべて既定経路で有効):

| 箇所 | 判定 |
|---|---|
| `src/presolve/rowsingleton.rs:49` | `value < lb - TOL \|\| value > ub + TOL` → infeasible (**今回の発火点**) |
| `src/presolve/foldfixed.rs:54-56` | 全項固定の行の残差 `\|folded\| <= TOL` (Eq) / `folded >= -TOL` (Le) |
| `src/presolve/propagate.rs:157` `bounds_inconsistent` | `lb > ub + PROPAGATE_EPS` |
| `src/presolve/propagate.rs:306` `propagate_split` | `min_activity > b + PROPAGATE_EPS` |
| `src/presolve/propagate.rs:644` `propagate_equalities` | `min_activity > b + EPS \|\| max_activity < b - EPS` |
| `src/presolve/propagate.rs:748` `propagate_equalities` | 締めた直後の `lb[k] > ub[k] + EPS` |

対照的に `ineqsingleton.rs:158`、`parallelrows.rs:98`、`colsingleton.rs:188` は既に `TOL * (1 + |x|)` の相対形。

なぜ ken-07/11/13 では出ないか: 同じ構造だが、桁落ち量 (ulp(|a_ik ub_k|)/|a_ik|) が偶然 1e-9 を超えなかっただけ
(§5 の掃引で ken-07/11/13 にも相対 1e-16 級の際どい判定が 77〜187 件ある)。ken-18 では ub の伝播値 2.27e8 が大きく、
`|a_ik| = 0.019` が小さいので 1.4e-8 になった。

## 5. 他の問題での発動状況 (前処理のみ、既定設定)

別 clone に「違反が相対 1e-6 以内の判定 (ok/INFEASIBLE)」を全て出すフックと、前処理直後に止める `DBG_PRESOLVE_ONLY` を入れ、
Netlib93 (`.netlib_cache/mps`) + Kennington 16 (`/home/user/kenn`) + Mittelmann 11 (`/home/user/mitt`) を掃引した
(`/tmp/claude-0/-home-user-enomoto-solver/.../scratchpad/sweep_base.jsonl`)。

- 前処理で infeasible になったのは **ken-18 のみ**。他の 119 問は前処理を通過。
- 際どい判定 (相対違反 ≤ 1e-6、判定は ok) が出た問題と件数:
  - Kennington: ken-07 77、ken-11 173、ken-13 187、ken-18 1135 (+ INFEASIBLE 1)。cre-*/osa-*/pds-* は 0。
  - Netlib: bandm 2、capri 5、czprob 6、fffff800 2、finnis 5、forplan 1、nesm 1、shell 7、ship04s 1、ship08s 5、ship12l 2、ship12s 4、vtp.base 1 (他は 0)。
  - Mittelmann: 0。
  - これらの相対違反の最大値は 3.2e-16 (ken-13/ken-07 でも 3e-16 以下) で、単純な 1〜2 ulp の丸め。
    ken-18 の 4.0e-14 だけが桁落ち由来で 2 桁大きく、絶対 1e-9 を超えた。
- スケール後の有限境界の最大値 (絶対許容誤差の危険度の目安): pds-02..100 9.3e9、fome13 5.5e9、dfl001 5.1e9、
  shell 1.9e9、agg/agg2/agg3 0.8〜1.1e9、ken-11 5.8e8、neos 5.3e7、pilot.we 3.9e7、sierra 3.6e7、cre-* 1.5〜1.8e7、
  pilot87 1.2e7、ken-07 1.0e8、ken-13 8.8e7。これらでは境界値 1 ulp が 1e-7〜1e-6 に達し、
  絶対 1e-9 の判定は「桁落ちが 1 回でも起きれば誤判定」の距離にある (今回たまたま発火していないだけ)。

## 6. 対処法

試作 (別 clone、`DBG_FIX=1` で有効化) は (A) の 5 箇所すべてを `tol * (1 + |量|)` の相対形にしたもの。
結果: **ken-18 optimal、obj = -52217025287.396805 (HiGHS と一致)、8.9 s**。同じ掃引 (前処理のみ) で
他の 119 問の前処理の結論 (feasible/infeasible) と、前処理後のスケール後境界の最大値に変化なし (120 問すべて完走、タイムアウトなし)。

### (A) 判定側の許容誤差を相対化 (推奨、最小変更)

- `src/presolve/rowsingleton.rs:49`: `let tol = TOL * (1.0 + value.abs());` で `value < lb - tol || value > ub + tol`。
  通った場合は `value` を `[lb, ub]` にクランプして固定する (HiGHS `HPresolve::singletonRow` と同じ扱い)。
- `src/presolve/foldfixed.rs:54-56`: 残差の許容誤差を `TOL * (1 + max(|rhs|, max_j |v_j x_j|))` に。
- `src/presolve/propagate.rs:157` (`bounds_inconsistent`)、`:748`: `PROPAGATE_EPS * (1 + |lb|)`。
- `src/presolve/propagate.rs:306`、`:644`: `PROPAGATE_EPS * (1 + max(|finite_sum|, |b|))`。
- 既定経路への影響: 実行不能の見逃しは「違反が相対 1e-9 未満」の場合に限られ、実質なし。
  縮約の内容 (どの列を固定するか) は変わらないので、Netlib/Mittelmann の反復回数・時間はほぼ不変のはず (要 A/B)。
  なお `PROPAGATE_EPS` は更新幅の閾値 (`candidate < ub - EPS`) にも使われているが、そちらは今回の件と無関係なので触らなくてよい。

### (B) 伝播側で桁落ちを避ける / 誤差分だけ緩める

- `propagate.rs:335/701/726`: `l_s = finite_sum_inf - contrib_k` の代わりに、行の項数が小さいとき (例 ≤ 8) は
  k を除いた和を直接計算する。または誤差見積もり `err = eps_mach * (|finite_sum_inf| + |contrib_k|)` を
  `candidate` に `err/|a_ik|` だけ緩める (下限候補なら引く、上限候補なら足す)。HiGHS も
  `impliedRowBounds` からの含意境界を「元の境界を強めるときだけ、しかも feastol で緩めて」使う。
- 影響: 伝播で得る境界がわずか (1e-14 相対) 緩くなるだけで、縮約は変わらない。ただし (A) なしでは
  他の丸め (行 2990 の 3e-16 級) には無防備なので、(A) と併用が望ましい。

### (C) rowsingleton の値を HiGHS 流にクランプ (A の一部だが単独でも ken-18 は直る)

- `rowsingleton.rs:49`: 境界からのはみ出しが「実行可能性許容誤差」(例 `PRIMAL_FEAS_TOL` 1e-7 相対) 以内なら
  境界値へクランプして固定、超えれば infeasible。ken-18 のはみ出し 1.4e-8 (相対 4e-14) は余裕で吸収される。
- 影響: 固定値が最大 1e-7 だけ動くが、後処理の値も同じなので目的関数値には影響しない。

### (D) 伝播で得た境界の信頼度を下げる (構造的)

- eqprop/propagate が締めた境界 (元の箱制約でないもの) は「含意境界」として別に持ち、rowsingleton/foldfixed の
  実行不能判定には元の境界だけを使う (HiGHS の `implColLower/Upper` と `col_lower_/upper_` の区別)。
  含意境界は縮約の候補選び (dualfix、ineqsingleton など) にだけ使う。
- 影響: 最も安全だが変更範囲が広い (lb/ub を読む全段)。今回の修正としては過剰。

### (E) 回避策 (設定のみ、恒久策ではない)

- `ENOMOTO_T_PROPAGATION_PASSES=1` (等式伝播が 1 パスになり ken-18 は直るが、他問題の縮約が減る)、
  または `ENOMOTO_T_RUIZ_ITERS=1` (スケーリングを弱める)。いずれも既定にする価値はない。

推奨: (A) + (C) を既定 on にし、(B) の「誤差見積もりで緩める」を propagate_equalities に入れる。
その上で `python scripts/...` の Netlib A/B と Kennington 16 問 (ken-18 を含む) で回帰確認。

## 7. 参考: 使ったファイル

- 計装 clone: `/home/user/prof_ken18` (HEAD bcd59a2 + eprintln、`DBG_TRACE_COL`、`DBG_NEARMISS`、`DBG_PRESOLVE_ONLY`、`DBG_FIX`)、venv `/home/user/venv_ken18`。
- 54de561 のビルドも同 clone/venv で行った (checkout 後に再ビルド)。
- 掃引スクリプト・結果: `/tmp/claude-0/-home-user-enomoto-solver/2b9d1d8c-debd-5f7e-86fa-9ac3ff5926a1/scratchpad/{sweep.py,sweep_base.jsonl,sweep_fix.jsonl}`。
