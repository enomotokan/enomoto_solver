# HiGHS に解けて ENOMOTO に解けない問題: 調査・手当て・評価 (2026-10-23)

依頼文は `next_sessions2.md` (依頼 I〜L)。前の回 (依頼 A〜H) の結果は `next_sessions_results.md`。
数はどれも子の機械 (4 コア) での 4 スレッド・60 秒。目的値の不一致 (HiGHS の最適値と違う「最適」) は、調査・計測のどのセッションでも 0 件。

## 1. 対象の問題 (ba7f18b の計測、既定 4 回)

HiGHS が 60 秒以内に証明し、ENOMOTO の既定が 1 回以上証明できなかった問題: mzzv42z・physiciansched6-2・roll3000・timtab1 (0/4)、
10teams・hypothyroid-k1・h80x6320d・beasleyC3・neos-960392・mc11 (3/4)。前の回で扱っていないもの・残ったものを依頼 I〜L にした。

## 2. 調査の結果 (依頼 I〜L)

| 依頼 | 原因 | 手当て (切り替え) | 効果 | 既定にする候補か | ブランチ |
|---|---|---|---|---|---|
| I 10teams | 遅いのは解。下界は 0.5 秒で 924 に届くが、根のカットが 924 のまま 12〜15 ラウンド (6〜8 秒) 回る。924 を出す根のサブ MIP (RRC/LP face) が上限 約 3.7 秒にぎりぎりで、外れると木で 25〜60 秒。目的値の刻みの判定も費用のある固定の連続列で None になっていた | `ENOMOTO_MIP_CUT_INTOBJ_STALL=1` (目的値が整数刻みなら、下界が上がった後 3 ラウンド動かなければカットのループを止める) | 3 回ずつ: 既定 10.1・60 (936)・38.4 → 5.4・10.9・5.4 秒、mixoc 10.3・25.6・26.9 → 5.5・5.3・5.4 秒 | はい (働くのは 10teams と app1-1 だけ、app1-1 も同じか速い) | inv/10teams-20261023 (c18447e) |
| J hypothyroid | 根の LP を 2 回解く (前処理の LP による固定の最初の LP が期限 3 秒で毎回打ち切られて捨てられ、根で基底なしで 5〜7 秒かけて解き直す)。根のカットの組の運で、打ち切り値つきの根のプロービングが証明に届かず再スタートする道もある | `ENOMOTO_MIP_LP_PROBE_FINISH=1` (期限 4 倍 + 行の状態と DSE 重みも入れた基底を渡す。根の LP は 1 反復 0.2 秒)、`ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES=3`、`ENOMOTO_MIP_EARLY_RENS=1` | 3 回ずつ: 3 つ組で 既定 24.8〜26.3 → 21.2〜22.1 秒、mixoc 25.2〜26.1 → 21.0〜21.9 秒 (HiGHS 19.4)。FINISH+PASSES だけでは向きが揃わない | EARLY_RENS は不可 (neos-960392・mc11 が時間切れ、cvs16r128-89 悪化)。FINISH+PASSES=3 は未評価 (下の計測に入れていない。LP_PROBE_FIRST_FULL と重なる) | inv/hypothyroid-20261023 (cbd5415) |
| K physbound | 根のカットのループが全体の上限 (時間の 10% ≒ 5.6 秒) で 1〜2 ラウンドで止まる。面の頂点の分離の LP 2 本が 1 ラウンド 2.3 秒を使い採用 0 本。1 回の LP の上限 1.2 秒は要る 1.1〜1.5 秒と重なり 7 回中 5 回時間切れ | `ENOMOTO_MIP_CUT_STRETCH` (下界が伸びる間は上限を延ばす (最大 30%)、LP の上限 5%、時間切れなら DSE 厳密で解き直す、面の LP が解けなければ面の分離をやめる) | 2 回ずつ、60 秒の下界: 既定 48587・48688 → 48957・48929、mixoc 48774・48587 → 49041・49216。証明 0 回。mzzv42z の根 +2.8 秒 | いいえ (60 秒で解けず、根が延びる問題がある) | inv/physbound-20261023 (5c8d92c) |
| L treeproof | 強分岐の予算 (25000 + ノード LP 反復の 0.125 倍) が HiGHS (10 万 + 0.5 倍) の約 1/4 で、主のスレッドが予算に当たり擬似費用だけで分枝。h80 の木 HiGHS 19 ノード / ENOMOTO 165〜307。h80 の再スタート回数は全整数列 6320 本の 2.5% を分母に 2 回目の根で 158 本以上固定されるかで決まる | `ENOMOTO_MIP_SB_BUDGET_HIGHS=1` (予算を HiGHS と同じに。サブ MIP では使わない) | 3 回ずつ: roll3000 の証明 既定 1/3 → 3/3 (31〜47 秒)、mixoc 3/3 → 3/3。h80 の木 13〜17 ノード、mixoc 29.4〜31.3 → 26.7〜27.1 秒 | そのままでは候補にしにくい (10teams・beasleyC3・neos-2978193-inde が遅い向き、1 回ずつ) | inv/treeproof-20261023 (ad70573) |

## 3. 評価 (40 問 + 16 問、2 組)

- 依頼 A〜L の切り替えを 1 本にまとめた: `inv/combo-20261023` (a261791、メモ `analysis/combo_20261023.md`)。ソースのぶつかりなし、cargo test 313 件通過。
- CAND_DEF = `WORKER_HALT=1 LATE_NO_RESOLVE=1 LP_PROBE_FIRST_FULL=1 GR_RESTART=1 FJ_NOSOL_EXTEND=1 CUT_STRONG_WIN5=1 CUT_INTOBJ_STALL=1` (頭に `ENOMOTO_MIP_`)。
- CAND_MIX = mixoc (`FJ_LMIP_MIX=1 SYM_ORIG_CUT=1`) + CAND_DEF (FJ_NOSOL_EXTEND=2) + `HEUR_MU_ROLE_OBS=1`。
- 計測: 8 本のセッションに問題を分け (`bench_subsets.txt`)、各セッションが同じ機械で def7 (既定)・cand7 (CAND_DEF)・mixoc7・mixcand7 (CAND_MIX) を 2 組 (HLm: この順、HLn: 逆順)。
  生の結果はブランチ `inv/combo-bench-20261023-s1`〜`-s8` の `benchmarks/*_a261791_*`。集計は `agg_combo.py`。
  (s4 は最初に組 3 の問題を測ったので、組 4 を測り直した。組 3 の問題は 2 台で測ったことになる。)

問題 56 (40 問のうち 40)
| 設定 | 組 | 40 問の解けた数 | 40 問の SGM 時間 (shift 10) | 全問の解けた数 | 主ギャップの和 | 解なし | HiGHS が 60 秒で解く問題のうち解けた数 | 不一致 |
|---|---|---|---|---|---|---|---|---|
| def7 | HLm | 26 | 20.61 | 36 | 1.462 | 1 | 30/36 | 0  |
| cand7 | HLm | 26 | 20.49 | 37 | 1.929 | 1 | 31/36 | 0  |
| mixoc7 | HLm | 25 | 22.06 | 36 | 1.331 | 1 | 31/36 | 0  |
| mixcand7 | HLm | 26 | 20.98 | 37 | 0.717 | 0 | 31/36 | 0  |
| def7 | HLn | 26 | 20.94 | 38 | 1.974 | 1 | 32/36 | 0  |
| cand7 | HLn | 26 | 20.30 | 37 | 1.697 | 1 | 31/36 | 0  |
| mixoc7 | HLn | 26 | 21.09 | 38 | 0.328 | 0 | 32/36 | 0  |
| mixcand7 | HLn | 26 | 21.07 | 36 | 0.681 | 0 | 30/36 | 0  |

問題ごと (2 組とも同じ向き: 主ギャップ差 >= 0.01、解けた/解けない、両方解けて時間 1.5 倍)
- cand7 が def7 より良い (2):
  - 10teams (cand7 ['opt 6.5 924.0', 'opt 5.7 924.0'] / def7 ['opt 14.0 924.0', 'opt 25.2 924.0'])
  - supportcase26 (cand7 ['tim 60.1 1776.7565080992854', 'tim 60.1 1772.5102968481026'] / def7 ['tim 60.0 1806.4799868580044', 'tim 60.1 1802.2337756067384'])
- cand7 が def7 より悪い (1):
  - mc11 (cand7 ['tim 60.0 11689.0', 'tim 60.0 11689.0'] / def7 ['opt 50.9 11689.0', 'opt 52.0 11689.0'])
- mixcand7 が mixoc7 より良い (1):
  - mzzv42z (mixcand7 ['tim 60.1 -20500.0', 'tim 60.1 -20540.0'] / mixoc7 ['tim 60.1 -20032.0', 'tim 60.1 -20270.0'])
- mixcand7 が mixoc7 より悪い (1):
  - mc11 (mixcand7 ['tim 60.0 11702.0', 'tim 60.0 11692.0'] / mixoc7 ['opt 45.2 11689.0', 'opt 43.9 11689.0'])
- mixcand7 が def7 より良い (6):
  - neos-950242 (mixcand7 ['opt 8.7 4.0', 'opt 9.2 4.0'] / def7 ['opt 17.3 4.0', 'opt 16.0 4.0'])
  - 10teams (mixcand7 ['opt 6.1 924.0', 'opt 9.9 924.0'] / def7 ['opt 14.0 924.0', 'opt 25.2 924.0'])
  - 30n20b8 (mixcand7 ['tim 60.1 353.0', 'tim 60.1 353.0'] / def7 ['tim 60.1 None', 'tim 60.8 502.0'])
  - cvs16r128-89 (mixcand7 ['tim 60.1 -91.0', 'tim 60.1 -91.0'] / def7 ['tim 62.1 -83.0', 'tim 61.2 -58.0'])
  - graph20-20-1rand (mixcand7 ['tim 60.1 -9.0', 'tim 60.1 -9.0'] / def7 ['tim 61.0 -8.0', 'tim 60.9 -8.0'])
  - supportcase26 (mixcand7 ['tim 60.0 1764.0178743456222', 'tim 60.0 1755.5254467363513'] / def7 ['tim 60.0 1806.4799868580044', 'tim 60.1 1802.2337756067384'])
- mixcand7 が def7 より悪い (1):
  - mc11 (mixcand7 ['tim 60.0 11702.0', 'tim 60.0 11692.0'] / def7 ['opt 50.9 11689.0', 'opt 52.0 11689.0'])


HiGHS が 60 秒で証明する 36 問のうち、どれかの設定で 1 回以上落とした問題 (2 組で証明できた回数):

| 問題 | 既定 | 既定+候補 | mixoc | mixoc+候補 |
|---|---|---|---|---|
| beasleyC3 | 1/2 | 2/2 | 1/2 | 2/2 |
| neos-960392 | 1/2 | 2/2 | 1/2 | 2/2 |
| neos-2978193-inde | 2/2 | 2/2 | 2/2 | 1/2 |
| mc11 | 2/2 | 0/2 | 2/2 | 0/2 |
| roll3000 | 0/2 | 0/2 | 1/2 | 0/2 |
| mzzv42z・physiciansched6-2・timtab1 | 0/2 | 0/2 | 0/2 | 0/2 |

時間の上限の超過 (時間切れの回のうち 60.5 秒を超えた回): 既定 12/34 (最大 62.33)、既定+候補 1/34 (60.89)、mixoc 9/34 (62.03)、mixoc+候補 0/35 (60.2)。

### mc11 の悪化の原因 (inv/mc11bisect-20261023、acaa6fa、メモ `analysis/mc11bisect_20261023.md`)

- 原因は `CUT_STRONG_WIN5` だけ。1 つだけ足すと 2 回とも時間切れ、CAND_DEF から WIN5 だけ外すと根の下界が戻る。ほかの 6 つは 1 つ足しても 2 回そろって悪くならない。
- 仕組み: mc11 は 25 ラウンド目で「強いカット」の判定の境目 (5 ラウンドの伸び 720、閾値 105)。WIN5 だと通って第 2 段階と HiGHS 流のカット管理に入り、根のヒューリスティクスの終わりが 8.4 → 14.5 秒に延び、背景のカット後の下界が 25〜40 低くなる。
- 直した形 `CUT_STRONG_WIN5=nofc` (固定費用のネットワーク (fc_relax) では今までの判定): mc11 単独 51.9・23.4 秒、CAND_DEF の WIN5 を nofc にして 35.6・45.5 秒、どちらも最適。beasleyC3 は 36.0・50.0 秒で最適 (WIN5 の効果は残る)。56 問ではまだ測っていない。

## 4. まとめ (事実と推測)

事実:
- CAND_DEF・CAND_MIX を足すと、2 組とも同じ向きに良くなったのは 10teams (I)・supportcase26、mixoc では mzzv42z の値 (F)。HiGHS が解く問題では beasleyC3 (H)・neos-960392 (E) が 2/2 になった。時間の上限の超過 (A) はほぼなくなった。
- 2 組とも同じ向きに悪くなったのは mc11 だけ (原因は CUT_STRONG_WIN5)。
- 40 問の解けた数・SGM 時間は、候補の有無で揺れの範囲 (26/26 対 26/26、SGM 差 0.1〜1 秒)。
- mzzv42z・physiciansched6-2・timtab1 は、どの設定でも 60 秒で証明できない。roll3000 は SB_BUDGET_HIGHS (候補外) でだけ証明が増えた。

推測:
- CUT_STRONG_WIN5 を =nofc にすれば mc11 の悪化は消え、候補の害は見当たらなくなる見込み (56 問での確認はまだ)。

## 5. 既定にする候補の切り替え

| 切り替え | 依頼 | 判断 |
|---|---|---|
| `ENOMOTO_MIP_WORKER_HALT=1` | A | 候補。超過がほぼなくなる |
| `ENOMOTO_MIP_LATE_NO_RESOLVE=1` | A | 候補 (時間切れ後の連続変数の研磨がなくなる点だけ注意) |
| `ENOMOTO_MIP_LP_PROBE_FIRST_FULL=1` | E | 候補。neos-960392 が 2/2 に |
| `ENOMOTO_MIP_CUT_INTOBJ_STALL=1` | I | 候補。10teams が 2 組とも速い |
| `ENOMOTO_MIP_GR_RESTART=1` | F | 候補 (害なし。mzzv42z の値は mixoc 側で改善) |
| `ENOMOTO_MIP_FJ_NOSOL_EXTEND=1` (既定) / `=2` (mixoc) | D | 候補 (害なし。この計測の機械では physiciansched6-2 の解なしが起きず、効果は見えない) |
| `ENOMOTO_MIP_CUT_STRONG_WIN5` | H | **=1 は不可 (mc11 が 4/4 で時間切れ)。=nofc を 56 問で測ってから** |
| `ENOMOTO_MIP_HEUR_MU_ROLE_OBS=1` | C | mixoc と組で候補 (bppc4-08 は 54 が 4/4。害なし) |
| `ENOMOTO_MIP_SB_BUDGET_HIGHS=1` | L | 候補外 (roll3000・h80 に効くが、ほかが遅い向き)。56 問で別に測る価値あり |
| `ENOMOTO_MIP_TREE_POOL_BASIS=1` | G | 候補外 (効果が揺れの範囲) |
| `ENOMOTO_MIP_LP_PROBE_FINISH=1` + `ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES=3` | J | 未評価 (LP_PROBE_FIRST_FULL と重なる。どちらかに寄せてから測る) |
| `ENOMOTO_MIP_CUT_STRETCH` | K | 候補外 |
| `ENOMOTO_MIP_EARLY_RENS=1` | J | 不可 |

## 6. 残り

- CUT_STRONG_WIN5=nofc を入れた CAND_DEF・CAND_MIX で 56 問を 2 組測り直す (inv/mc11bisect-20261023 を combo に merge してから)。
- mzzv42z (解は改善、証明なし)・physiciansched6-2 (根の下界 48587〜49216 対 HiGHS 49324)・timtab1 (別の機械では HiGHS も 80 秒)・roll3000 (SB_BUDGET_HIGHS でのみ証明) は、まだ HiGHS との差が残る。
- uccase12 で再スタート後の解 11507.41 が後処理で実行不能として捨てられる件 (依頼 A の別件) は未調査。
