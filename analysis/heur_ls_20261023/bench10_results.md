# 40 問 + 16 問の計測 (56ec351、直した候補の組 fix9pe と前処理の手当て、2026-10-24)

依頼文は `bench10_instructions.md`。ビルドは claude/enomoto-solver-heuristic-survey-xjlnol の 56ec351 (既定は mixoc)。
8 本のセッションに問題を分け、同じ機械で 3 つの設定を 2 組 (HLs: def10→fixpre10、HLt: 逆順)。4 スレッド・60 秒。
生の結果は `inv/combo10-bench-20261024-s1`〜`-s8` の `benchmarks/*_56ec351_*`、集計は `agg_combo10.py`。目的値の不一致は 0 件。

| 名前 | 中身 (頭の ENOMOTO_MIP_ を略す) |
|---|---|
| def10 | 何も付けない (既定 = mixoc) |
| fix10 (fix9pe) | WORKER_HALT LATE_NO_RESOLVE LP_PROBE_FINISH LP_PROBE_ROW_BASIS POSTSOLVE_REPAIR GR_RESTART FJ_NOSOL_EXTEND=2 CUT_STRONG_WIN5=nofc CUT_INTOBJ_STALL HEUR_MU_ROLE_OBS SYM_SKIP_FIXED=empty CUT_AGG=f RESTART_BASIS + ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES=3 |
| fixpre10 | fix10 + CLIQUE_EXTEND_ROWS_FIRST PROBE_EQUIV_FIX |

問題 56 (40 問のうち 40)
| 設定 | 組 | 40 問の解けた数 | 40 問の SGM 時間 (shift 10) | 全問の解けた数 | 主ギャップの和 | 解なし | HiGHS が 60 秒で解く問題のうち解けた数 | 不一致 |
|---|---|---|---|---|---|---|---|---|
| def10 | HLs | 24 | 21.87 | 35 | 0.198 | 0 | 29/36 | 0  |
| fix10 | HLs | 24 | 21.01 | 34 | 0.314 | 0 | 29/36 | 0  |
| fixpre10 | HLs | 25 | 21.39 | 36 | 0.411 | 0 | 30/36 | 0  |
| def10 | HLt | 25 | 21.66 | 36 | 0.324 | 0 | 31/36 | 0  |
| fix10 | HLt | 25 | 21.28 | 37 | 0.130 | 0 | 31/36 | 0  |
| fixpre10 | HLt | 26 | 21.14 | 37 | 0.317 | 0 | 31/36 | 0  |

問題ごと (2 組とも同じ向き: 主ギャップ差 >= 0.01、解けた/解けない、両方解けて時間 1.5 倍)
- fix10 が def10 より良い (4):
  - drayage-100-23 (fix10 ['opt 6.0 103333.87407302856', 'opt 5.1 103333.87407302856'] / def10 ['opt 16.0 103333.87407302856', 'opt 17.8 103333.87407302856'])
  - drayage-25-23 (fix10 ['opt 10.2 101282.64701843262', 'opt 19.7 101282.64701843262'] / def10 ['opt 28.9 101282.64701843262', 'opt 54.0 101282.64701843262'])
  - neos-960392 (fix10 ['tim 60.1 -234.0', 'opt 35.9 -238.0'] / def10 ['tim 60.3 -228.0', 'tim 60.6 -228.0'])
  - neos-911970 (fix10 ['opt 15.8 54.76', 'opt 20.2 54.760000000000005'] / def10 ['opt 32.1 54.76', 'opt 38.3 54.76'])
- fix10 が def10 より悪い (0):
- fixpre10 が def10 より良い (0):
- fixpre10 が def10 より悪い (2):
  - physiciansched6-2 (fixpre10 ['tim 61.0 52330.00000000004', 'tim 62.6 52330.00000000004'] / def10 ['tim 62.7 50825.000000000015', 'tim 62.4 50825.000000000015'])
  - 30n20b8 (fixpre10 ['tim 60.2 353.0', 'tim 60.3 353.0'] / def10 ['tim 62.0 302.0', 'tim 60.8 302.0'])
- fixpre10 が fix10 より良い (2):
  - h80x6320d (fixpre10 ['opt 58.1 6382.09905', 'opt 43.0 6382.09905'] / fix10 ['tim 60.0 6382.0990499935915', 'tim 60.3 6391.755905989111'])
  - neos-1456979 (fixpre10 ['tim 60.2 188.0', 'tim 60.1 189.0'] / fix10 ['tim 60.1 210.0', 'tim 60.1 195.0'])
- fixpre10 が fix10 より悪い (2):
  - physiciansched6-2 (fixpre10 ['tim 61.0 52330.00000000004', 'tim 62.6 52330.00000000004'] / fix10 ['tim 60.2 49324.0', 'tim 60.2 50826.0'])
  - bppc4-08 (fixpre10 ['tim 60.1 55.0', 'tim 60.1 55.0'] / fix10 ['tim 60.1 54.000000000000036', 'tim 60.0 53.99999999999997'])


時間の上限の超過 (時間切れの回のうち 60.5 秒を超えた回): def10 11/41 (最大 62.7 秒)、fix10 0/41 (60.28 秒)、fixpre10 8/39 (62.56 秒)。

## 読み取り

事実:
- fix10 (fix9pe) は既定より 2 組とも悪い問題が 0、良い問題が 4 (drayage-100-23・drayage-25-23・neos-960392・neos-911970)。
  40 問の解けた数は 24/25 で同じ、SGM 時間は 21.87/21.66 -> 21.01/21.28 秒、時間の上限の超過は 11 回 -> 0 回。
- 前の候補の組 (cand9、d20cf31) で悪くなった neos-960392・qiu・misc07・nw04・dcmulti・mik-250-20-75-4・hypothyroid-k1 は、fix10 で既定並みかそれ以上に戻った。
- fixpre10 (前処理の手当てを足す) は既定より良い問題が 0、悪い問題が 2 (physiciansched6-2 52330・30n20b8)。fix10 と比べても h80x6320d・neos-1456979 が良く、physiciansched6-2・bppc4-08 が悪く、混ざる。mzzv42z の値も fix10 より良くない。uccase12 は fixpre10 で 11507.90 (fix10 は 11507.41)。

推測:
- fix10 (fix9pe) は既定にする候補。前処理の手当て (CLIQUE_EXTEND_ROWS_FIRST・PROBE_EQUIV_FIX) は今の形では候補にしない。
