# pilot87 の誤 infeasible (作業 #8: 正しさのバグ) 調査レポート

対象: enomoto_solver ブランチ claude/compassionate-lamport-v8hmvy、HEAD 3338daa (作業 #5 M1/M1'/M2/M3 入り) と 57cd940 (案 E を試した時点、作業 #5 前)。
コードは書き換えていない (計装・試作は別 clone /home/user/prof_p87 (HEAD) と worktree /home/user/prof_p87_old (57cd940)、venv は /home/user/venv_p87, /home/user/venv_p87o)。

## 要約

- **再現した**。`perturb_costs` (src/simplex.rs:756) で費用 0 の列の摂動を 0 にすると (案 E 相当)、57cd940 では pilot87 が 1.0 s で
  **infeasible** を返す。現 HEAD でも作業 #5 の雑音判定を切る (`ENOMOTO_T_NOISE_C=0`) と**同じ反復 (iter 4802, 行 48) で infeasible** になる。
  既定の HEAD (雑音判定 on) では M1' (雑音ピボットの除外、25 反復・509 候補) が経路を変えて optimal に着く (13.4 s、既定 3.8 s、目的関数値は
  HiGHS と相対 4.4e-10 ずれる) が、M1' だけを切って M2 (Farkas 証明) を残すと M2 が結論を拒否して (certified=false) not_solved 側に流れる。
  **作業 #5 は症状を隠している (誤 infeasible → optimal/not_solved) だけで根本原因は残っている。**
- **原因の段**: B 段 (切片問題) の双対単体法主ループ、**BFRT 使い切り** (`site=bfrt_exhausted`、HEAD src/simplex/slope_intercept_dual.rs:3957-3994)。
  行 48 (基底変数 = スラック 4942、下限 0・上限 ∞) の `x_B = -7.34`、Eligible 4 列の総フリップ容量 3.22 < 7.34 で「真の実行不能」と結論。
  しかしその時点の基底は**数値的に特異寸前** (行 48 の `rho = B^-T e_r` は正確 (非零 5 個、最大 1) なのに、`rho^T (b - N x_N)` から求まる
  `x_B[48]` は **+2.70** で実行可能。被約費用は `|d| ≈ 1e8〜3e10` (費用は O(1))。LU 解の `x_B` が壊れている)。
- **機序**: 費用 0 の列 (pilot87 では構造列 4486 本のうち 3843 本) の被約費用が厳密に 0 になり、双対比率テストが**全候補同点 (ratio 0)・
  双対ステップ 0 の退化ピボット**を延々と繰り返す (4783 反復中 3629 が退化、既定の摂動ありは 0)。同点では `(ratio, j)` 順で
  `k_star` が決まり Harris 窓 (後ろ向きだけ) が最大 |α| を選ぶが、選ばれた行の Eligible が極小 α しか持たないことが多く、
  絶対許容誤差 `TOL = 1e-9` しかないピボット判定 (`chuzc1_filter_one`、HEAD :6293) が `|α_q| = 2.3e-6 (列最大 7.9)`、`4.7e-8 (列最大 2.1e4)` の
  ピボットを受け入れる。`‖B^-1 行‖∞` が 1e2 (iter 109) → 1e4 (290) → 1e6 (1607) → 1e8 (2483) → 1e10 (4504) → 1e21 (4611) と単調に悪化し、
  iter 4613 で被約費用が 1e8 に飛び (`DEBUG_EXT_DUAL_CHECK`)、iter 4802 の `x_B`・`w_r` は雑音。既定の摂動ありでは `‖B^-1 行‖∞` は 1e6 未満。
  「更新済み分解からは結論しない → 再分解して再試行」のガード (HEAD :3957) は同じ特異寸前の基底を分解し直すだけなので効かない。
- **同じ経路で崩れる問題** (Netlib 93 問、HEAD ビルド): 費用 0 の列の摂動 0 + 雑音判定 off で **cycle・perold・pilot.ja・pilot87・pilotnov が
  infeasible (誤)、pilot・tuff が not_solved**。雑音判定 on (既定) では cycle・pilot.ja・pilot・pilotnov・tuff が not_solved (pilot87・perold は optimal)。
  摂動を全列 0 にすると grow22・scsd8 も not_solved に加わる。摂動 0.1 倍・費用 0 の列だけ 0.1 倍・既定は 93/93 OK。
  摂動を 1e-3 倍 (絶対 5e-10) にしても pilot87・perold は崩れる (HARRIS_RATIO_TOL 1e-7 以下の差は同点と同じ)。
  HiGHS は同じ 7 問を `dual_simplex_cost_perturbation_multiplier=0` (摂動なし) でも全問 optimal (反復 +0〜60%)。

## 1. 再現手順

```
git clone /home/user/enomoto_solver /home/user/prof_p87 && cd /home/user/prof_p87 && git checkout claude/compassionate-lamport-v8hmvy
python -m venv /home/user/venv_p87 && /home/user/venv_p87/bin/pip install maturin highspy numpy
cp -r /home/user/base_repo/target target && VIRTUAL_ENV=/home/user/venv_p87 /home/user/venv_p87/bin/maturin develop --release
# 57cd940 は git worktree add /home/user/prof_p87_old 57cd940 で同様 (venv_p87o)
```

`perturb_costs` に調査用 env を追加 (両ビルド。HEAD 側は src/simplex.rs:791 の `xpert` 計算直後):
`ENOMOTO_X_PERTURB_FACTOR` (全列の摂動を係数倍)、`ENOMOTO_X_PERTURB_ZERO_COST_FACTOR` (費用 0 の列だけ係数倍 = 0 で案 E 相当)。
57cd940 では案 D の `shrink` 適用後 (`let xp = xpert[j] * shrink;`) に掛ける。案 E の本来の実装 (費用非零列の摂動総量が既に目標を超えるとき
費用 0 の列の縮小率が 0 に張り付く) と同じく、pilot87 では費用 0 の列の摂動が 0 になる (`PERTURB n_orig=4486 n_pert_zero_cost=3843 ratio=8.0e-2`)。

実行: `python /tmp/claude-0/.../scratchpad/run1.py .netlib_cache/mps/pilot87.mps` (iters.py と同じ `benchmark_highs._load_lp`/`_build_our_model`、`solve(root_solver=None)`)。

| ビルド | 設定 | status | obj (HiGHS 301.71034733310734) | 時間 | 主ループ反復 |
|---|---|---|---|---:|---:|
| 57cd940 | 既定 | optimal | 301.71034733310734 | 3.8 s | 6069 |
| 57cd940 | 費用 0 の列の摂動 0 | **infeasible** (誤) | — | 1.0 s | 4783 (iter 4802 で結論) |
| 57cd940 | 費用 0 の列の摂動 1e-3 倍 / 1e-6 倍 | **infeasible** / not_solved | — | 1.4 / 8.1 s | |
| HEAD 3338daa | 既定 | optimal | 301.71034733310734 | 3.8 s | 6072 |
| HEAD | 費用 0 の列の摂動 0 | optimal | 301.7103474645602 (相対 4.4e-10) | 13.4 s | 24645 (noise_pivot_iters=25, cands=509, infeas_guard=0) |
| HEAD | 同 + `ENOMOTO_T_NOISE_C=0` (作業 #5 off) | **infeasible** (誤、同じ iter 4802 r=48) | — | 1.0 s | |
| HEAD | 同 + `ENOMOTO_T_NOISE_MIN_SQRT_W=1e300` (M1' off、M2 on) | not_solved 経路 (certificate r=48 `t=2.14 ∈ [-3.78, inf]` → 拒否、以後 singular basis の bailout 多数) | — | | |
| HEAD | 摂動なし (全列 0) | **not_solved** | — | 276 s | |

## 2. 原因の特定 (計装の結果)

### 2.1 結論している段と場所

`DEBUG_EXT_INFEASIBLE`:
```
DEBUG_EXT: stage A skipped (S empty)            ← A 段なし、B 段 (phase=B) の主ループ
DEBUG_EXT_INFEASIBLE: site=bfrt_exhausted iter=4802 r=48 basis_r=4942 d_dir=1 w_r=(7.337635464259787,0) n_candidates=4 cum=(3.2242137843673646,0) remaining_m_side=67
```
HEAD の該当箇所: src/simplex/slope_intercept_dual.rs:3950-3994 (`let Some(k_star) = k_star else { ... return Some(SimplexResult { status: Status::Infeasible, .. }) }`)。
`lu.update_count() == 0` (直前に再分解済み) なので :3957 のガードは通過し、57cd940 では即 Infeasible、HEAD では :3976 `infeas_check!` (M2) に進む。
cleanup/polish/主単体法への引き継ぎ/前処理は関係ない (主ループ内で終わる)。

### 2.2 その時点の行・列の値 (57cd940 に `ENOMOTO_X_DUMP_INFEAS` を追加して出力)

```
XDUMP site=bfrt_exhausted iter=4802 r=48 basis_r=4942 d_dir=1 w_r=(7.3376,0) x_b=(-7.3376,0) lower=Some(0) upper=None(∞)
      noise_feasible=false stuck_row=Some(25) safe_pivot=false bland=false ban_active=false banned=[3812] n_touched=10 n_sorted=4 phase=B lu_updates=0
XDUMP rho_max=1.000e0 rho_nnz=5 t=rho^T b=2.140378e0 g_basis_r=1.000000e0 max|g_basic(other)|=3.5e-18 d_r=0
  j     種別   g=rho^T a_j  状態   d_j        lb  ub      幅      hat_alpha  判定
  3810  struct -9.505e-4    Upper  -2.98e8    0   345     345     +9.5e-4    符号不適 (非候補)
  3812  struct +1.682e-3    Upper  +5.28e8    0   345     345     -1.68e-3   CAND (ban 済みだが ban_active=false)
  3813  struct +2.920e-3    Upper  +9.16e8    0   345     345     -2.92e-3   CAND
  3814  struct -5.401e-3    Lower  -1.70e9    0   201.76  201.76  -5.40e-3   CAND
  3861  struct -7.812e-4    Lower  -5.42e8    0   700.35  700.35  -7.81e-4   CAND
  3863  struct +1.382e-3    Lower  +9.60e8    0   700.35  700.35  +1.38e-3   符号不適
  3864  struct +2.400e-3    Lower  +1.67e9    0   700.35  700.35  +2.40e-3   符号不適
  3865  struct -4.439e-3    Upper  -3.08e9    0   409.57  409.57  +4.44e-3   符号不適
  4491  slack  +5.401e-3    Lower  +1.70e9    0   0 (固定)                    幅 0
  4500  slack  +4.439e-3    Lower  +3.08e9    0   0 (固定)                    幅 0
  4756  slack  +5.000e-2    Lower  +1.57e10   0   ∞       ∞       +5.0e-2    符号不適
  4945  slack  +1.000e0     Lower  +2.85e10   0   ∞       ∞       +1.0       符号不適
```
- 候補 4 列の容量 `Σ|g_j|×幅 = 0.58+1.01+1.09+0.55 = 3.22 = cum` で、判定自体はこの `x_B`・`nb_status` に対して正しく実装されている。
- しかし **`x_B[48]` が間違っている**: `rho` は正確 (基底列の `rho^T a_j` は最大 3.5e-18) なので `x_B[48] = rho^T b − Σ_{非基底} g_j x_j
  = 2.140 − (−0.328 + 0.580 + 1.007 − 1.818) = +2.70 ≥ 0` (実行可能)。LU 解の `x_B` は `-7.34`。同じ LU で `rho` は正確で `x_B` が
  10 ずれるのは、`x_B` の他成分が 1e19〜1e21 (トレースの `w_r`)、`cond(B) ≳ 1e21` で後退安定な解の成分誤差 `≈ cond·eps·‖x_B‖` が O(10) になるため。
- 被約費用 `|d_j| = 3e8〜3e10` (費用 O(1)、摂動 5e-7): `y = B^-T c_B` も壊れている。比率は `hat_c = max(σ d_j, 0) = 0` に潰れて全候補比 0。
- HEAD の M2 証明 (`infeasibility_certified`, :926) は同じ `rho` で `t=2.14 ∈ [lo=-3.78, hi=∞]` → 不成立 (正しい)。

### 2.3 そこに至る過程 (57cd940 に反復ごとのトレース `ENOMOTO_X_TRACE` を追加)

| 指標 | 費用 0 の列の摂動 0 (誤 infeasible) | 既定の摂動 (optimal) |
|---|---:|---:|
| 主ループ反復 | 4783 (iter 4802 で結論) | 6069 |
| 退化ピボット (`|d_q| < 1e-12`, θ_d = 0) | **3629 (76%)** | **0** |
| Harris 窓に同点 2 候補以上 | 3289 | 466 |
| `|α_q| < 1e-6` のピボット | 5 (最小 2.7e-8) | 0 (最小 4.3e-6) |
| `|α_q| < 1e-8 × 行最大` / `< 1e-10 × 列最大` | 68 / 76 | 0 / 0 |
| `‖rho‖∞ > 1e4 / 1e6 / 1e8 / 1e10 / 1e18` 初出 | iter 290 / 1607 / 2483 / 4504 / 4611 | 579 / — / — / — / — |
| 最大双対違反 (`DEBUG_EXT_DUAL_CHECK`) | 9.4e13 (iter 4613 で初めて > 1) | 0.024 |

典型的な退化ピボット (Eligible 全体が同点で、極小 α しか無い行を選んでいる):
```
iter=287  r=886  q=486  alpha_q=-2.262e-6 (列最大 7.9)  dj_q=1.4e-21 theta_d=-6e-16 n_cand=24 k_star=23 n_tie_win=24 rho_max=2.4  → iter 290 で rho_max 8.4e4
iter=4608 r=1377 q=418  alpha_q=+4.657e-8 (列最大 2.1e4) dj_q=0        n_cand=3  k_star=2  n_tie_win=3  rho_max=3.7e9 lu_upd=0 (再分解直後)
iter=4609 r=1246 q=181  alpha_col_max=2.3e20 ...  iter=4611 r=120 alpha_q=-6.2e4 rho_max=1.965e21 → iter 4613 d_max=1.2e8
```
- 費用 0 の列 (3843 本) の `d_j` は厳密に 0 → 比率 `hat_c/|α| = 0` で全候補同点 → `theta_d = 0` の退化ピボットの連鎖。
  `(ratio, j)` 順のヒープでは `k_star` は添字順で決まり、Harris パス 2 (:3997-4010、後ろ向きの窓のみ) は同点の `[0, k_star]` から最大 |α| を選ぶが、
  選ばれた行の Eligible がすべて極小 α (行がほぼ従属) のことが多い。ピボットの受け入れ条件は `chuzc1_filter_one` の絶対値 `|α| > TOL = 1e-9` だけで、
  行最大・列最大に対する相対判定は無い (`STUCK_ROW_MIN_PIVOT = 1e-7` は `stuck_row` かつ `safe_pivot` のときだけ、M1' は `sqrt(w_r) > 1e9` になってから)。
  こうして `B^-1` の成長が積み上がり、`x_B`・`d` が雑音になり、雑音の `x_B` を「実行不能行」として選び、BFRT 使い切りが「証明」になる。
- 既定の摂動ありでは `d_j ≠ 0` で比率が分かれ、極小 α の列は比 `d_j/|α_j|` が大きく後ろに並ぶので、退化ピボットが 0 回、`‖rho‖∞ < 1e6`。
- 摂動が **1e-3 倍 (絶対 5e-10)** でも pilot87 は infeasible: `HARRIS_RATIO_TOL = 1e-7`・`LEX_REL_TOL = 1e-9` の下では 5e-10 の差は同点と同じ。
  0.1 倍 (5e-8) なら 93/93 OK。摂動の有無で正しさが変わるのは判定側のバグ (HiGHS は摂動 0 でも同じ 7 問を optimal にする、下記)。

### 2.4 HEAD (作業 #5) が既定で pilot87 を救っている仕組みと限界

- 既定 HEAD + 費用 0 の列の摂動 0: `noise_pivot_iters=25 noise_pivot_cands=509 infeas_guard=0` → M1' (雑音ピボット除外、`sqrt(w_r) > 1e9` の行だけ) が
  iter 4500 付近以降の経路を変え、結論の場面に来ない。反復は 6072 → 24645 (4 倍)、目的関数値のずれ 4.4e-10。**経路の偶然**で、
  cycle・pilot.ja・pilot・pilotnov・tuff は同設定で not_solved。
- M1' を切って M2 だけ残すと、M2 は正しく拒否 (`certified=false`) するが、その後の LU 閾値引き上げ → 再分解は「特異基底」で失敗を繰り返し
  (`refactorize returned None` 多数)、not_solved になる。つまり **M2 は誤 infeasible を not_solved に変えるだけで、解には至らない**。
- 作業 #5 は「特異基底に耐える」目的で導入された安全網としては機能しているが、本件の根本 (退化ピボットの連鎖で基底を壊す・壊れた `x_B` を信じる) は未対処。

## 3. 同じ経路で崩れる問題 (Netlib 93 問、HEAD ビルド、`scratchpad/sweep.py`、300 s 打ち切り、HiGHS と相対 1e-6 で照合)

| 設定 | OK | 誤 status (問題: status) |
|---|---:|---|
| 既定 / 既定 + `NOISE_C=0` | 93/93 | — |
| 費用 0 の列の摂動 0 (Z0) | 88/93 | cycle・pilot.ja・pilot・pilotnov・tuff: **not_solved** (pilot87 13 s optimal、perold optimal) |
| Z0 + `ENOMOTO_T_NOISE_C=0` (作業 #5 off) | 86/93 | **infeasible**: cycle・perold・pilot.ja・pilot87・pilotnov; not_solved: pilot・tuff |
| 摂動なし (P0) | 86/93 | not_solved: cycle・grow22・pilot.ja・pilot・pilot87・scsd8・tuff |
| P0 + `NOISE_C=0` | 84/93 | **infeasible**: perold・pilot.ja・pilot87・pilotnov; not_solved: cycle・grow22・pilot・scsd8・tuff |
| 摂動 0.1 倍 (P01) / 同 + `NOISE_C=0` / 費用 0 の列だけ 0.1 倍 (Z01) | 93/93 | — |

57cd940 ビルド (Z0) での各問題の終わり方と退化の度合い (`ENOMOTO_X_TRACE` 集計):

| 問題 | 終わり方 (57cd940, Z0) | 反復 | 退化ピボット | `‖rho‖∞ > 1e8` 初出 | `|α_q| < 1e-6` |
|---|---|---:|---:|---:|---:|
| pilot87 | infeasible (bfrt_exhausted iter 4802) | 4783 | 3629 | 2483 | 5 |
| pilot.ja | infeasible (bfrt_exhausted iter 963, cum 1820 < w_r 6975) | 905 | 905 (100%) | 571 | 3 |
| pilotnov | infeasible (bfrt_exhausted iter 1267, cum 28.1 < w_r 32) | 1260 | 1227 | 846 | 8 |
| perold | not_solved (refactorize → singular basis) | 1121 | 1042 | 189 | 17 |
| cycle | not_solved (singular basis) | 22301 | 22078 | 3009 | 4 |
| pilot | not_solved (singular basis) | 39766 | 37642 | 11848 | 2 |
| tuff | not_solved (max_iters=20000 exhausted, bland_mode) | 20000 | 20000 (100%) | — | 0 |

HEAD ビルド Z0 の not_solved の内訳: pilot.ja は `infeas_guard site=bfrt_exhausted iter=963 r=308 ... guard=false` (M2 拒否) の後 170 s 迷走、
pilot は `sqrt_w=1.6e20 guard=true`、pilotnov は `site=eligible_empty iter=8727` を M2 が拒否、cycle・tuff は反復上限/特異基底。

HiGHS (highspy、`solver=simplex, simplex_strategy=1 (dual)`) は `dual_simplex_cost_perturbation_multiplier=0` でも 7 問すべて optimal
(反復: pilot87 8369→8780、pilot.ja 1287→1645、pilotnov 2154→3222、perold 1149→1179、cycle 3726→3010、tuff 152→175、pilot 4815→7916)。
HiGHS は同点の候補群 (workGroup) 全体から最大 |α| を選び、ピボット許容誤差を更新回数で 1e-9→1e-8→1e-7 と上げ、双対値が許容誤差内に来たら
費用シフト (`shift_cost`) で厳密退化を避ける。

## 4. 試した対処 (57cd940 ビルド、Z0) と結果

| 試作 | pilot87 | pilot.ja | pilotnov | perold | cycle | tuff | pilot |
|---|---|---|---|---|---|---|---|
| 案 B: `|α_q| < 1e-7 × 行最大` なら行を一時タブー (`stuck_row_tabooed`) | infeasible (26364 回拒否、iter 32817、w_r=4.4e12) | not_solved | infeasible | not_solved | not_solved | not_solved | **optimal** |
| 同 1e-9 | infeasible | not_solved | not_solved | not_solved | not_solved | not_solved | not_solved |
| 案 B2: 候補の絶対閾値 `TOL` を 1e-7 に | not_solved | not_solved | not_solved | not_solved | not_solved | not_solved | **optimal** |
| 費用 0 の列の摂動 1e-3 倍 | infeasible | optimal | optimal | not_solved | optimal | optimal | optimal |
| 費用 0 の列の摂動 1e-6 倍 | not_solved | not_solved | optimal | not_solved | optimal | optimal | optimal |

→ ピボットの大きさの閾値だけでは足りない (行を外しても次の退化行で同じことが起き、`‖rho‖∞` は 1e8 に達する)。厳密退化の連鎖そのものを止めるか、
壊れた基底から回復する仕組みが要る。

## 5. 対処法 (複数、file:line は HEAD 3338daa)

1. **[最小の守り] 摂動量に下限を置く** — src/simplex.rs:791 `xpert = (1+r)(|c_j|+1) base`。将来また「縮める」変更をするときも、列ごとの摂動が
   `COST_PERTURB_BASE × max_abs_cost` の 0.1 倍 (Netlib で崩れない下限) を割らないようにし、`debug_assert!` で守る。案 E のように 0 に張り付く実装を禁止する。
   期待効果: 既定経路への影響なし (既定は下限より上)。ただし判定側のバグは残る。

2. **[結論の正しさ] 実行不能の結論を `x_B` に依存させない** — :3950-3994 (`bfrt_exhausted`) と :3743-3800 (`eligible_empty`)。M2 (:3008 `infeas_check!`、
   :926 `infeasibility_certified`) は既に `rho` による Farkas 証明を要求している。**M2 を `noise_on` (NOISE_C > 0) と独立に常時 on** にする
   (現状 `if noise_on { ... }` の中にあり、`ENOMOTO_T_NOISE_C=0` で誤 infeasible が復活する)。さらに証明が成り立たないときの回復を
   「LU 閾値引き上げ → 再分解」(基底自体が特異寸前なので効かない) から **基底の再構築** に変える: 例) `rho^T (b − N x_N)` と `x_B[r]` の不一致
   (今回 2.70 vs −7.34) を検出したら、その行の基底変数をスラック基底へ戻す/クラッシュ基底から双対をやり直す/摂動を掛け直して続行 (5. 参照)。
   期待効果: 誤 infeasible は無くなる (既に HEAD 既定では無い)。既定経路への影響: 証明が成り立つ結論は変わらない。

3. **[根本] 厳密退化の連鎖を止める: 費用シフト (HiGHS `HEkkDual::updateDual`/`shift_cost` 流)** — 双対ステップ `theta_d = dj_q / alpha_q` (:4928) が
   厳密 0 の反復が連続する (例: 連続 50 回、または `|d_q| ≤ 1e-12`) とき、入る列 (と同点候補) の `active_cost` を `±(1+r)·base` だけ動的にシフトして
   `d_j` を非零にし、以降は通常の摂動と同じく cleanup/polish (`polish_with_true_bounds`) で真の費用に戻す。`active_cost`/`d` の増分維持は
   `dual_active_costs` (:2297) と同じ配列なのでシフトを加えるだけで済む。期待効果: 摂動 0 でも既定と同じ挙動 (退化 0 回) に近づき、
   `‖B^-1‖` の成長を防ぐ。既定経路への影響: 既定では厳密退化が pilot87 で 0 回なので原則不変 (他問題は `|d_q|<1e-12` の頻度を `ENOMOTO_PROF_PHASES` の
   `DEGENERATE_PIVOTS` で確認して閾値を決める)。

4. **[根本の補助] ピボット選択の安定化** — (a) Harris パス 2 (:3997-4010) を HiGHS `chooseFinal` と同じ「同点群 (ratio が `HARRIS_RATIO_TOL` 内) 全体」
   から最大 |α| を選ぶ形に拡張 (現状 `k_star` より後ろの同点候補は見ない。フリップは `sorted[..k_star]` から `q` を除いた集合で、`theta_d` は同点なので双対実行可能性は保たれる)。
   (b) 候補の絶対閾値 `TOL=1e-9` (:6293 `chuzc1_filter_one`) を HiGHS 流に更新回数で 1e-9/1e-8/1e-7 と上げ、加えて `|α_q| < 1e-7 × max_j |a_p[j]|` なら
   その行を捨てて `refactor`/別行へ (試作の案 B は単独では不十分だが 3. と併用する価値あり)。期待効果: 極小ピボットの受け入れが減る。
   既定経路への影響: 同点 (pilot87 既定で 466 反復) の選択が変わり得るので A/B が必要。

5. **[回復] 「摂動なしで詰まったら摂動を入れてやり直す」** — 実行不能の証明不成立 (`infeas_check!` の `!certified`)、`refactorize` の特異、
   bland モードの反復上限 (`max_iters exhausted`) のいずれかで、`active_cost` に改めて摂動を加え (`perturb_costs` を係数 1 で再適用)、
   スラック基底または現基底から双対を再開する (HiGHS の「数値的トラブル時は摂動付きで再解」と同じ)。既定で 93/93 通る摂動に戻すので
   Z0/P0 の 7〜9 問は optimal になる見込み。既定経路への影響なし (既定ではこの分岐に来ない)。

6. **[検出] `x_B` の整合性検査を安価に** — 再分解直後 (`resync_x_b` の後) に、選んだ行 `r` について `x_B[r]` と `rho^T (b − N x_N)`
   (PRICE で `a_p` は既に手元にあるので `Σ_j a_p[j]·x_j` で O(touched)) の差が `1e-6·max(1,|x_B[r]|)` を超えたら基底を数値的に壊れたものとして
   5. の回復へ。M2 の証明と違い「行が実行不能か」ではなく「`x_B` が信用できるか」を直接見る。

推奨の組み合わせ: 2 (M2 を常時 on + 回復) と 3 (費用シフト) を主、1 を守り、4(b)・6 を補助。修正後の回帰は `scratchpad/sweep.py` の
Z0N/P0N 設定 (雑音判定 off で摂動 0) が最も厳しい (7〜9 問が誤 status)。

## 付録: 計装ファイル

- /home/user/prof_p87/src/simplex.rs (HEAD + `ENOMOTO_X_PERTURB_FACTOR` / `ENOMOTO_X_PERTURB_ZERO_COST_FACTOR` / `ENOMOTO_X_PERTURB_STRUCT_ONLY`)
- /home/user/prof_p87_old/src/simplex.rs, src/simplex/slope_intercept_dual.rs (57cd940 + 同 env、`ENOMOTO_X_DUMP_INFEAS` (結論時点の列一覧)、
  `ENOMOTO_X_TRACE` (反復ごとの α_q・θ_d・‖rho‖∞・d の最大・同点数)、試作 `ENOMOTO_X_REL_PIVOT_TOL`・`ENOMOTO_X_CHUZC_ABS_TOL`)
- /tmp/claude-0/-home-user-enomoto-solver/2b9d1d8c-debd-5f7e-86fa-9ac3ff5926a1/scratchpad/: run1.py (単発)、sweep.py + sweep_*.json (Netlib 93 問)、
  tr.py (トレース集計)、old_dump.txt (2.2 の生データ)、old_trace*.txt (2.3、各問題)、highs_nopert.py
