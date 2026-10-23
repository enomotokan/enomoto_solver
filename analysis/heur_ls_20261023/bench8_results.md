# 40 問 + 16 問の測り直し (acaa6fa、CUT_STRONG_WIN5=nofc・SB_BUDGET_HIGHS、2026-10-23)

依頼文は `bench8_instructions.md`。ビルドは inv/mc11bisect-20261023 の acaa6fa (inv/combo-20261023 a261791 + `CUT_STRONG_WIN5=nofc`)。
8 本のセッションに問題を分け (`bench_subsets.txt`)、同じ機械で 6 つの設定を 2 組 (HLo: def8→mixcand8、HLp: 逆順。mixcandsb8 は後から足したので両組とも最後)。
4 スレッド・60 秒。生の結果は `inv/combo8-bench-20261023-s1`〜`-s8` の `benchmarks/*_acaa6fa_*`、集計は `agg_combo8.py`。目的値の不一致は 0 件。

| 名前 | 中身 (頭の ENOMOTO_MIP_ を略す) |
|---|---|
| def8 | 何も付けない |
| cand8 | WORKER_HALT=1 LATE_NO_RESOLVE=1 LP_PROBE_FIRST_FULL=1 GR_RESTART=1 FJ_NOSOL_EXTEND=1 CUT_STRONG_WIN5=nofc CUT_INTOBJ_STALL=1 |
| candsb8 | cand8 + SB_BUDGET_HIGHS=1 |
| mixoc8 | FJ_LMIP_MIX=1 SYM_ORIG_CUT=1 |
| mixcand8 | mixoc8 + cand8 (FJ_NOSOL_EXTEND=2) + HEUR_MU_ROLE_OBS=1 |
| mixcandsb8 | mixcand8 + SB_BUDGET_HIGHS=1 |

問題 56 (40 問のうち 40)
| 設定 | 組 | 40 問の解けた数 | 40 問の SGM 時間 (shift 10) | 全問の解けた数 | 主ギャップの和 | 解なし | HiGHS が 60 秒で解く問題のうち解けた数 | 不一致 |
|---|---|---|---|---|---|---|---|---|
| def8 | HLo | 25 | 20.74 | 37 | 1.387 | 1 | 32/36 | 0  |
| cand8 | HLo | 26 | 20.02 | 37 | 2.312 | 1 | 31/36 | 0  |
| candsb8 | HLo | 26 | 19.42 | 38 | 3.065 | 1 | 32/36 | 0  |
| mixoc8 | HLo | 26 | 19.11 | 37 | 0.324 | 0 | 31/36 | 0  |
| mixcand8 | HLo | 25 | 19.88 | 37 | 1.103 | 0 | 32/36 | 0  |
| mixcandsb8 | HLo | 26 | 19.52 | 39 | 1.190 | 0 | 33/36 | 0  |
| def8 | HLp | 27 | 19.28 | 39 | 1.413 | 1 | 33/36 | 0  |
| cand8 | HLp | 26 | 18.91 | 37 | 2.468 | 1 | 31/36 | 0  |
| candsb8 | HLp | 26 | 20.37 | 37 | 3.893 | 2 | 31/36 | 0  |
| mixoc8 | HLp | 25 | 19.58 | 37 | 0.467 | 0 | 32/36 | 0  |
| mixcand8 | HLp | 26 | 20.24 | 38 | 1.167 | 0 | 33/36 | 0  |
| mixcandsb8 | HLp | 26 | 20.49 | 38 | 1.306 | 0 | 32/36 | 0  |

問題ごと (2 組とも同じ向き: 主ギャップ差 >= 0.01、解けた/解けない、両方解けて時間 1.5 倍)
- cand8 が def8 より良い (1):
  - physiciansched6-2 (cand8 ['tim 60.1 49623.999999899', 'tim 60.1 49623.999999899'] / def8 ['tim 60.5 50826.00000000001', 'tim 60.5 50826.00000000001'])
- cand8 が def8 より悪い (2):
  - mc11 (cand8 ['tim 60.0 11689.0', 'tim 60.1 11689.0'] / def8 ['opt 58.5 11689.0', 'opt 54.4 11689.0'])
  - uccase12 (cand8 ['tim 60.2 902356.7943993622', 'tim 60.3 902356.7943993622'] / def8 ['tim 61.2 11507.90312455934', 'tim 61.2 11507.903124559338'])
- candsb8 が def8 より良い (1):
  - physiciansched6-2 (candsb8 ['tim 60.2 49623.999999899', 'tim 60.2 49523.99999979904'] / def8 ['tim 60.5 50826.00000000001', 'tim 60.5 50826.00000000001'])
- candsb8 が def8 より悪い (3):
  - uccase12 (candsb8 ['tim 60.3 902356.7943993622', 'tim 60.2 902356.7943993622'] / def8 ['tim 61.2 11507.90312455934', 'tim 61.2 11507.903124559338'])
  - cvs16r128-89 (candsb8 ['tim 60.0 -54.0', 'tim 60.0 -54.0'] / def8 ['tim 60.6 -89.0', 'tim 60.1 -87.0'])
  - neos-1456979 (candsb8 ['tim 60.1 295.0', 'tim 60.1 None'] / def8 ['tim 60.2 203.0', 'tim 60.1 185.0'])
- candsb8 が cand8 より良い (0):
- candsb8 が cand8 より悪い (3):
  - blp-ar98 (candsb8 ['tim 60.0 6958.444756800002', 'tim 60.0 6958.444756800002'] / cand8 ['tim 60.1 6431.315953600001', 'tim 60.1 6414.329894400001'])
  - cvs16r128-89 (candsb8 ['tim 60.0 -54.0', 'tim 60.0 -54.0'] / cand8 ['tim 60.1 -89.0', 'tim 60.1 -91.0'])
  - neos-1456979 (candsb8 ['tim 60.1 295.0', 'tim 60.1 None'] / cand8 ['tim 60.1 197.0', 'tim 60.1 241.0'])
- mixcand8 が mixoc8 より良い (2):
  - 30n20b8 (mixcand8 ['tim 60.1 302.0', 'tim 60.1 302.0'] / mixoc8 ['tim 60.3 353.0', 'tim 61.0 353.0'])
  - neos-1456979 (mixcand8 ['tim 60.0 187.0', 'tim 60.1 201.0'] / mixoc8 ['tim 60.1 201.0', 'tim 60.1 216.0'])
- mixcand8 が mixoc8 より悪い (1):
  - uccase12 (mixcand8 ['tim 60.2 817923.0323252019', 'tim 60.2 817923.0323252019'] / mixoc8 ['tim 61.3 11507.903124559341', 'tim 61.1 11507.903124559341'])
- mixcandsb8 が mixcand8 より良い (2):
  - neos-1456979 (mixcandsb8 ['tim 60.0 179.0', 'tim 60.1 189.0'] / mixcand8 ['tim 60.0 187.0', 'tim 60.1 201.0'])
  - noswot (mixcandsb8 ['opt 17.8 -41.0', 'opt 37.5 -41.0'] / mixcand8 ['tim 60.0 -41.0', 'tim 60.0 -41.0'])
- mixcandsb8 が mixcand8 より悪い (3):
  - blp-ar98 (mixcandsb8 ['tim 60.0 7044.824931200001', 'tim 60.1 6497.010739200002'] / mixcand8 ['tim 60.1 6398.978411199998', 'tim 60.1 6404.225332800001'])
  - cvs16r128-89 (mixcandsb8 ['tim 60.1 -91.0', 'tim 60.0 -91.0'] / mixcand8 ['tim 60.1 -92.0', 'tim 60.0 -95.0'])
  - timtab1 (mixcandsb8 ['tim 60.0 781644.9999998399', 'tim 60.0 780545.9999997799'] / mixcand8 ['tim 60.0 764771.99999978', 'opt 47.7 764771.9999997802'])
- mixcand8 が def8 より良い (4):
  - physiciansched6-2 (mixcand8 ['tim 60.1 49424.0', 'tim 60.1 49425.0'] / def8 ['tim 60.5 50826.00000000001', 'tim 60.5 50826.00000000001'])
  - 30n20b8 (mixcand8 ['tim 60.1 302.0', 'tim 60.1 302.0'] / def8 ['tim 60.7 None', 'tim 60.4 None'])
  - cvs16r128-89 (mixcand8 ['tim 60.1 -92.0', 'tim 60.0 -95.0'] / def8 ['tim 60.6 -89.0', 'tim 60.1 -87.0'])
  - graph20-20-1rand (mixcand8 ['tim 60.1 -9.0', 'tim 60.1 -9.0'] / def8 ['tim 60.5 -8.0', 'tim 61.3 -8.0'])
- mixcand8 が def8 より悪い (1):
  - uccase12 (mixcand8 ['tim 60.2 817923.0323252019', 'tim 60.2 817923.0323252019'] / def8 ['tim 61.2 11507.90312455934', 'tim 61.2 11507.903124559338'])


HiGHS が 60 秒で証明する 36 問のうち、どれかの設定で 1 回以上落とした問題 (2 組で証明できた回数):

| 問題 | HiGHS | def8 | cand8 | candsb8 | mixoc8 | mixcand8 | mixcandsb8 |
|---|---|---|---|---|---|---|---|
| mc11 | 55.1 秒 | 2 | 0 | 1 | 1 | 2 | 2 |
| neos-950242 | 25.8 秒 | 2 | 2 | 1 | 2 | 2 | 2 |
| roll3000 | 46.3 秒 | 0 | 0 | 1 | 0 | 0 | 1 |
| timtab1 | 59.9 秒 | 1 | 0 | 0 | 0 | 1 | 0 |
| mzzv42z・physiciansched6-2 | 50.1・10.8 秒 | 0 | 0 | 0 | 0 | 0 | 0 |

時間の上限の超過 (時間切れの回のうち 60.5 秒を超えた回): def8 7/36 (最大 61.3)、mixoc8 5/38 (62.29)、候補のある 4 つは 0 (最大 60.35)。

## 読み取り

事実:
- 40 問の解けた数・SGM 時間は、どの設定も揺れの範囲 (25〜27 問、19.1〜20.7 秒)。主ギャップの和は mixoc 系が小さい (0.32〜1.31、既定系は 1.39〜3.89)。
- 前回 (a261791) 4/4 で悪かった mc11 は、mixcand8 で 2/2 に戻った。cand8 は 0/2 だが、既定も 58.5・54.4 秒で上限の際にある (HiGHS 55.1 秒)。
- 候補の組で 2 組とも悪いのは **uccase12** (cand8・candsb8 902356.8、mixcand8・mixcandsb8 817923.0、既定・mixoc 11507.90)。最初の解が 60.1 秒にしか出ない。調べている (inv/uccase12-20261023)。
- SB_BUDGET_HIGHS は roll3000 を 1/2 解くが、cvs16r128-89 (−54 対 −89〜−91)・blp-ar98・neos-1456979 を 2 組とも悪くする (既定側)。mixoc 側では noswot・neos-1456979 が良く、blp-ar98・cvs16r128-89・timtab1 が悪い。
- 前回良かった beasleyC3・neos-960392・neos-2978193-inde は、今回は既定でも全部解けたので差が出ない。

推測:
- uccase12 は LATE_NO_RESOLVE で「後処理で捨てた解を前処理なしの解き直しで拾い直す」道がなくなったため (後処理で実行不能になること自体が元の不具合)。
- SB_BUDGET_HIGHS は、強分岐の効く問題と効かない問題の差が大きく、そのままでは既定にしにくい。SB_AGREE (claude/enomoto-solver-heuristic-survey-xjlnol) で切り分けられるかを見ている。
