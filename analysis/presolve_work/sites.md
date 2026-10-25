# 前処理の中で時間で打ち切る箇所 (1e25b07、既定 = mixoc + fix9pe)

| # | 場所 (src/mip/mod.rs) | 今の上限 (既定) | 時計 | 何が変わるか | PRESOLVE_WORK での置き換え |
|---|---|---|---|---|---|
| 1 | `solve_problem_inner` の `lp_probe_secs` | 時間制限 × `ENOMOTO_T_MIP_LP_PROBE_TIME_FRAC` (0.05)。60 秒で 3 秒、20 秒で 1 秒、時間制限なしで 10 秒 | — | 下の 2〜5 の期限そのもの。時間制限で前処理の結果が変わる元 | 安全網 `ENOMOTO_T_MIP_PRESOLVE_WORK_LP_SECS` (30 秒、時間制限の半分まで) |
| 2 | `lp_probe` 最初の LP | 期限 `secs` × `ENOMOTO_T_MIP_LP_PROBE_FINISH_MULT` (4、`LP_PROBE_FINISH` 既定)。60 秒で 12 秒 | 壁時計 | 間に合わなければ固定なし・基底を根に渡さない (根で解き直し、着く頂点が変わる) | 反復の上限 4 × `_WORK_LP` / (行 + 列 + 非零) |
| 3 | `lp_probe` 固定を試す間 (`budget_left`) | 最初の LP からの経過 < `secs` (3 秒)。反復の上限 (2 × 最初の LP + 5000) と併用 | 壁時計 | 試す列の数 → 固定の数 → 前処理後の行・列と LP の値 (app1-1・neos-960392) | 合計の反復 < `_WORK_LP` / (行 + 列 + 非零) |
| 4 | `lp_probe` 子の LP・固定後の解き直し | 期限 `secs` | 壁時計 | 途中で止まった子の LP は「実行不能」と見なさない → 固定の数 | 残りの反復の上限 |
| 5 | `lp_probe` あきらめ (`giveup_secs`) | 固定なしで経過 > 最初の LP の時間 × 1 + 0.1 秒 | 壁時計 | 固定の出る前にやめるか | 反復 > 最初の LP の反復 + `_WORK_LP_GIVEUP` / (行 + 列 + 非零) |
| 6 | `lp_probe` 基底を渡すか | 最初の LP ≥ `ENOMOTO_T_MIP_LP_PROBE_BASIS_SECS` (1 秒) | 壁時計 | 形は変わらないが根の LP の基底・頂点 (以後の経路) | 最初の LP の反復 × (行 + 列 + 非零) ≥ `_WORK_LP_BASIS` |
| 7 | `lp_probe` `LP_PROBE_FINISH=late` (既定でない) | 最初の LP < `secs` | 壁時計 | 行の基底・DSE 重みを渡すか | 反復で判定 |
| 8 | `probe` 成果連動 (既定) | 何も得られない間 `ENOMOTO_T_MIP_PROBE_ADAPT_TIME0` 1 秒、得られた後 `_ADAPT_TIME` 3 秒 (+ 手間 2000 × 非零) | スレッドの CPU 時間 (`cap_clock`、`ENOMOTO_MIP_WALL_CAPS` で壁時計) | 試した列の数 → 固定・締め付け・含意 → 前処理後の形 (30n20b8・neos-860300・air03・eil33-2・hypothyroid-k1) | 手間 (走査した行の長さ + 更新した活動 + 試し 1 回 100) の上限 `_WORK_PROBE0`・`_WORK_PROBE` |
| 9 | `probe` 旧来 (サブ MIP、`NO_PROBE_ADAPTIVE`)・`PROBE_ADAPTIVE_SUBMIP` | `ENOMOTO_T_MIP_PROBE_TIME` 0.5 秒 / `_TIME_FOUND` 0.5 秒 / `_ADAPT_TIME_SUB` 0.5 秒 | CPU 時間 | サブ MIP の前処理 (求解中の RENS・RINS の結果) | `_WORK_PROBE_SUB` |
| 10 | `probe` 同値の 2 巡目 (`PROBE_EQUIV2`・`PROBE_CLIQUE=equiv`、既定でない) | 0.5 秒・見つからなければ 0.1 秒 | 壁時計 | 見つける同値な組 | `_WORK_EQUIV2` (と 1/5) |
| 11 | `add_symmetry_rows` → `symmetry::detect` | `ENOMOTO_T_MIP_SYM_TIME` 0.3 秒 (+ 試す組 2000)。`search` の中でも同じ期限 | 壁時計 | 生成元の数 → lex-leader の行・オービトープ (beasleyC3 は 190〜490 個で揺れた記録) | 手間 (細分化・確かめで読んだ非零 + 行 + 列) の上限 `_WORK_SYM` |

時間で打ち切らない (確認した) もの: `run_extended_mip` (presolve/ 以下、ラウンド数で打ち切り)、係数の強化、行の併合、
クリーク行の拡張 (1 行 1000 文字)、`DOMCOL` (組の数)、`ENUM_PRESOLVE` (行・失敗の数)、疎化、`DUPCOL`・`DUAL_BOUND2`。
presolve/ 以下の `Instant` は scaling.rs のテストだけ。

前処理は再帰で何度も呼ばれ (行の併合・LP による固定・プロービングの後のかけ直し)、8 は呼ばれるたびに上限を数え直す。
再スタート後の前処理 (求解中) も 8 を使う (2〜6 は最初の求解だけ)。
