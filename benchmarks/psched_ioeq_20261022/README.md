# in-out 分離を同値代入のある求解に限った計測 (2026-10-22)

ブランチ `inv/psched-ioeq-20261022` (bcb6a07)、4 コア・4 スレッド。

- `e2hl` = E (`PROBE_EQUIV2` `EQUIV2_CUTS` `EQUIV2_FJ` `EQUIV2_TAB`) + HL (`ROOT_SUBMIP_HL`)
- `e2iohleq` = e2hl + `ENOMOTO_MIP_INOUT_SEP=1 ENOMOTO_MIP_INOUT_EQ2_ONLY=1`

## (a) physiciansched6-2、60 秒 × 10 回 (交互)。最適値 49324

| | e2hl | e2iohleq |
|---|---|---|
| 解けた回数 | 0/10 | 0/10 |
| 時間 | 60.3–60.9 s (すべて時間切れ) | 60.3–60.9 s (すべて時間切れ) |
| 暫定解の値 | 50626 ×8, 49324 ×2 | 50626 ×6, 49324 ×3, 49724 ×1 |
| 最初の暫定解 | 7.19–7.91 s、値はすべて 50626 | 7.90–8.72 s、値はすべて 50626 |
| 時間切れの時点の下界 | 49023.7 ×2, 49157.3 ×4, 49265.7 ×3, 49299.0 | 49186.4, 49224.0 ×2, 49232.3, 49324.0 ×6 |
| 根の下界が 49324 に届いた回数 | 0 | 6 |
| ノード数 | 0,3,1,1,0,1,1,1,0,1 | 0,0,1,3,0,1,0,3,3,0 |

- e2iohleq の rep 0 は暫定解 49324・下界 49324 (差 0) なのに、状態は time_limit で返った。
- 根の下界が 49324 に届いた 6 回のうち、暫定解が 49324 だったのは rep 0 だけで、残る 5 回は 50626。
- 20 回とも返った解は実行可能で目的値も一致した。MIP: WARNING と異常終了はなし。

## (b) 55 問 × (newdef = 環境変数なし / e2iohleq)、10 秒、ENOMOTO_MIP_LOG=1

状態はどちらも optimal 16・time_limit 39。解が返った 50 件はすべて実行可能で、目的値も一致した (解なしは 30n20b8, neos-1456979, ns1116954, nursesched-sprint02, physiciansched6-2 で、どちらの設定でも同じ)。MIP: WARNING と異常終了はなし。

- 同値の代入 (EQUIV2) が主問題で働いた問題: beavma, graph20-20-1rand, physiciansched6-2, timtab1
- in-out の行が出た問題 (行数): beavma 4 (2 本), graph20-20-1rand 12 (16 本), timtab1 136 (16 本), physiciansched6-2 2 (内点の計算に失敗して 0 本)。括弧の本数は round 0 の alpha 0.5 で加わった切除平面。ほかの 51 問では in-out の行は出なかった。
- 根の RINS (HL) の行が出た問題: 10teams, app1-1, graph20-20-1rand, misc07, mzzv42z, neos-950242, supportcase26 (各 1 回)
- 主問題の前処理後の大きさが変わった問題: app1-1, beavma, drayage-100-23, drayage-25-23, fiber, gt2, h80x6320d, neos-860300, nw04, physiciansched6-2, qnet1, timtab1 (ログの最後の presolve 行で比べた)

目的値・状態が newdef と違った問題 (newdef → e2iohleq、状態はどれも time_limit のまま。最小化なので小さいほど良い):

| 問題 | 目的値 |
|---|---|
| 10teams | 976 → 1026 |
| assign1-5-8 | 213 → 214 |
| beasleyC3 | 760 → 770 |
| bppc4-08 | 56 → 62 |
| drayage-25-23 | 106481.6 → 104956.4 |
| gen-ip054 | 6858.91 → 6870.08 |
| h80x6320d | 6491.06 → 6485.11 |
| mas74 | 12101.9 → 11857.4 |
| mzzv42z | -1190 → -9108 |
| nw04 | 16976 → 36512 |
| pk1 | 11 → 13 |
| ran14x18-disj-8 | 3964 → 3840 |
| roll3000 | 14993 → 14308 |
| supportcase26 | 1916.88 → 1912.64 |
| timtab1 | 819238 → 885892 |

目的値の差には、newdef に E と HL がないことによる分も入っている。in-out の行が出たのは 4 問だけで、そのうち目的値が変わったのは timtab1 (悪化) だけ。
