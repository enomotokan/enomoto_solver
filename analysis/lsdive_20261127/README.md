# LSDIVE: SCIP の linesearchdiving に当たる木の中のダイビング (2026-11-27)

背景: analysis/t1primal_20261126・analysis/gap240_20261126 によると、SCIP は 30n20b8 の最初の解を木の中の linesearchdiving で 88 秒に見つける。
当方の 1 スレッドの最初の解は 255〜268 秒 (T1PRIMAL の L の def_t1)。当方はこの種類のダイビングを持っていなかった。

## 1. SCIP の linesearchdiving の中身と、当方のダイビングとの違い

ソース: SCIP master の `src/scip/heur_linesearchdiving.c` (選び方) と `src/scip/heuristics.c` の `SCIPperformGenericDivingAlgorithm` (枠)。

選び方 (`divesetGetScoreLinesearchdiving`): 列の根の LP 解 r (`SCIPvarGetRootSol`) と今の LP 解 v を比べる。
- v < r なら切り下げ、点数 = (分数部分 + ε) / (r − v)。v > r なら切り上げ、点数 = (1 − 分数部分 + ε) / (v − r)。
  根から今への直線を延ばしたとき、整数値に先に届く列ほど点数が小さい (SCIP は −点数 の最大を選ぶ)。
- その向きに自明に丸められる列 (`SCIPvarMayRoundDown/Up`) は点数を 1000 倍して後回し。v = r の列は点数無限大 (最後)。

| 項目 | SCIP linesearchdiving | 当方の木の中のダイビング (`Solver::dive`、solver.rs の「ダイビング」の段) |
|---|---|---|
| 列と向きの選び方 | 根の LP 解からの直線 (上の式) | 分数・ベクトル長・係数・擬費用・誘導・Farkas・衝突の 7 種 (+ 試作の objpscost)。**根の LP 解を使う種類が無かった** (根の LP 解も木へ残していなかった) |
| 呼ぶ時期 | 深さ 10 ごと、ずらし 6 (`HEUR_FREQ` 10・`FREQOFS` 6)。LP のプランジの後 | 深さを 10 で割った余りが 3 か 7 のノードで 1 回。種類は巡回で 1 つ (6〜7 種を順に) |
| 頻度・強さの決め方 | ダイビングごとに独立に呼ぶ (adaptivediving はさらにバンディットで公開の divesets から選ぶ) | 種類を巡回 (既定)。`ENOMOTO_MIP_DIVEQUOTA` (試作) は種類ごとの上限 |
| LP 反復の上限 | ダイビングごとに 0.05 × (解の数 + 1) / (呼んだ数 + 1) × ノードの LP 反復 + 1000 | 全種類で共有の 0.05 × (成功 + 1) / (呼んだ数 + 1) × ノードの LP 反復 + 1000 (同じ式を全体で 1 つ) |
| 目的値の打ち切り | 暫定解あり: 下界 + 0.8 × (打ち切り値 − 下界)。なし: 打ち切り値が無限なので制限なし | 同じ (暫定解なしは無限) |
| LP を解く頻度 | `lpsolvefreq` 0: 同じ LP 解の候補を続けて固定し、伝播で変わった境界が列の 15% を超えたか候補が尽きたら解き直す | 1 列固定するごとに解き直す (`ENOMOTO_T_MIP_DIVE_MULTI_DIV` でまとめて固定できるが既定 0) |
| 実行不能のとき | 1 段だけ戻して反対側 (`backtrack` TRUE) | 同じ列の反対側を試す。それより前へは戻らない (木の中は `ENOMOTO_T_MIP_NODE_DIVE_BACKTRACKS` 0) |
| 自明に丸められる列 | 1000 倍で後回し | lock 0 を同じく 1000 倍で後回し (当方の lock で判定) |

## 2. 試作 `ENOMOTO_MIP_LSDIVE` (既定オフ)

- `DiveKind::LineSearch` (src/mip/heuristics.rs) を足した。点数は SCIP と同じ式 (根と同じ値の列は 1e300 で最後)。
- 根の最後の LP 解を `Solver::ls_root_x` に残し (根の処理の終わり、`hcm_root_obj` を決める所。再スタート後は作り直した根のもの)、
  `TreeSnapshot` で木のスレッドへも渡す。
- 呼ぶ時期・LP 反復の上限・打ち切り・LP の扱いは既存の木の中のダイビングの枠のまま。巡回の種類の並びの最後に足すだけで、
  新しい閾値は作らない (頻度は巡回、強さは既存の成功率つきの反復の上限で決まる)。
