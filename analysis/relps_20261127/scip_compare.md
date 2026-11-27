# SCIP の reliability pseudo-cost branching (relpscost) と当方の分枝の選び方の比べ (2026-11-27、読むだけ・計測なし)

## 0. 前提と記号

- SCIP 側のソースは scipopt/scip の **master** (scratchpad/scip_src/、以下 `relps:行` は master の branch_relpscost.c)。
  比べの相手の計測 (treegap・sbadapt・sbdegen) は **SCIP 10.0.2** なので、v10.0.0〜v10.0.2 のタグの branch_relpscost.c も取って (scip_src/v1002/) 差分を見た。
  **既定値は 10.0.2 と master で同じ。ただし強分枝の予算の式だけが違う** (1.3 節)。ほかの式は同じ。
- 当方は src/mip/solver.rs (`select_branch_inner` = solver.rs:20416〜21603) と src/mip/pseudocost.rs。行番号は今の作業ツリー。
- 印: **[確認]** = ソースを読んで確かめたこと。**[推測]** = 確かめていない見立て。
- x = ノードの LP の反復の累計 (SCIP `SCIPgetNNodeLPIterations`、当方 `node_iters`)。s(v, a) = 1 − 1/(1 + v/a)。

---

## 1. 信頼度 (reliability) の決め方

### 1.1 表

| 項目 | SCIP (既定値・式・場所) | 当方 (既定値・式・場所) | 違い |
|---|---|---|---|
| 信頼度の閾値の範囲 | minreliable 1.0・maxreliable 5.0 (relps:83-84) | 1.0〜5.0 の固定値 (`base_thr`、solver.rs:20636) | 同じ |
| 閾値を動かす式 | prio = min(1, (maxsb − sb)/(sb+1)); prio = max(prio, (quot − sb)/(sb+1)); reliable = (1−prio)·1 + prio·5 (relps:1436-1439) | 同じ式 (solver.rs:20625-20626、20636)。ただし prio は 0 で下を切る | 式は同じ。**quot・maxsb の大きさが違う** (1.3 節) ので、同じ木の位置でも当方の閾値は早く 1 に下がる [推測: 予算が 1/4 のため] |
| 数える観測 | `SCIPgetVarPseudocostCountCurrentRun` の下・上の小さい方 (今の run だけ) (relps:1150-1153、1657-1660) | `min_observations` = n_up・n_down の小さい方 (pseudocost.rs:349)。再スタートは別の Solver なので実質「今の run」 | ほぼ同じ |
| 観測に数えるもの | 木の子の最初の LP の伸びと、強分枝の子 (重み 1)。**強分枝の子が反復上限で止まっても数える** (`WITH_LPSOLSTAT` は未定義なので relps:1981・1998 の除外は効かない) | 木の子の最初の LP (solver.rs:16094-16106)、強分枝の子で LP が最適まで解けたもの (solver.rs:21290 付近)。**反復上限で止まった子は数えない** (solver.rs:21311-21316、`ENOMOTO_MIP_PCL=a` の試作だけ数える) | 違う [確認]。当方は上限で止まる候補が信頼に届かず、何度も強分枝される |
| 片側が実行不能の候補 | 擬似費用を更新しない (`storesemiinitcosts` FALSE、relps:107、1979-2014) | 実行可能だった側は更新する。打ち切り値以上で最適だった子も観測に入れる (solver.rs:21290 付近) | 小さな違い |
| 単位あたりの値 | (objdelta + **1e-4**) / max(距離, **0.1**) の単純平均 (history.c:191-247、def.h:171-172 の pseudocosteps 0.1・pseudocostdelta 1e-4) | gain / 距離 (距離の下限 1e-9) を和と回数で持ち、**全体の平均と混ぜる**: w = min(1, 0.9 + 0.1·n/8) (pseudocost.rs:237、278-287、`ENOMOTO_T_MIP_MINREL` 既定 8 は混ぜる重みにだけ効く) | 違う。SCIP は伸び 0 の観測でも 1e-4 が乗り、擬似費用が 0 にならない (小数部が必ず少し効く) |
| 観測のない列の値 | 全体の平均 (glbhistory) (var.c:20753-20777) | 全体の平均 (pseudocost.rs:278-281) | 同じ |
| 相対誤差による信頼度 | `userelerrorreliability` FALSE、`usehyptestforreliability` FALSE (relps:102、105) = 既定は回数だけ | `ENOMOTO_MIP_PCL=c` (試作、既定オフ) で信頼区間 | 既定どうしは同じ (回数だけ) |
| **統計で強分枝を省く** | `skipbadinitcands` **TRUE** (relps:110、1804-1850)。下に詳述 | なし | **違う [確認]** |

### 1.2 skipbadinitcands (SCIP の既定で有効、当方にないもの)

relps:1804-1850。信頼できない候補 c を強分枝する直前に:

- いまの最良が強分枝の候補 (両側 valid) なら、`SCIPpscostThresholdProbabilityTest(c, frac, bestsbdown, 下)` か同じ上向きが真なら c を飛ばす。
  検定は var.c:21295-21359: その向きの観測が 2 回以上、平均 frac·pscost が閾値より小さく、正規分布で「閾値以下」の確率 ≥ 0.9 (clevel 2) なら真。
- いまの最良が擬似費用の候補なら、t 検定 (`SCIPsignificantVarPscostDifference`、片側、両者とも観測 2 回以上) で有意に悪ければ飛ばす (var.c:21229-21290)。
- **閾値に渡すのは `bestsbdown` = 子の LP の値そのもの (伸びではない)** (relps:1818-1821、bestsbdown は relps:2140 で down を代入、down は relps:1963 で max(down, lpobjval) の絶対値) [確認]。
  目的値が正で伸びより十分大きい問題 (mcsched ≈ 1.9e5、swath1 ≈ 340、dano3_5 ≈ 576) では、観測が 2 回以上ある向きを持つ候補は、平均の伸び ≪ 閾値なのでほぼ必ず飛ばされる [推測: 式からの見立て。意図か不具合かは分からない]。
  目的値が負 (pg −8680 など) では閾値が負になり、ほぼ飛ばさない。
- つまり SCIP は、そのノードで強分枝の最良が出た後は「観測 0〜1 回の候補」に強分枝を集中させ、2〜4 回の候補にはほとんど予算を使わない [推測]。当方は閾値 (1〜5) 未満の候補を全部同じに扱う。

### 1.3 予算 (信頼度の閾値の元)

| | SCIP 10.0.2 (計測の相手) | SCIP master | 当方 既定 |
|---|---|---|---|
| quot (prio の第 2 項) | 0.5·x | 0.5·x × 0.25 (run ≤ 3 の間) = 0.125·x | 0.125·x (`ENOMOTO_T_MIP_SB_QUOT`、solver.rs:20575) |
| maxsb | 0.5·x + 100000 **+ 根の強分枝の反復** (v10.0.2:1380) | 0.25·(0.5·x + 100000) **+ 根の強分枝の反復** (relps:1393-1405、`sbiterquotrestart` 0.25・`sbiterrestarts` 2、relps:88-91) | 0.125·x + 25000 (solver.rs:20576)。**根の強分枝の反復も `sb_iters` に入る** (除く仕組みは試作 `ROOTSB=..f` だけ、solver.rs:20329-20335) |
| 予算切れの扱い | ノードの始めで sb > maxsb なら強分枝なし (relps:1406-1408)。ノードの中では「候補の番号 < maxlookahead なら予算を超えても続ける」(relps:1027-1028) | 同左 | ノードの始め (`allow_sb`、solver.rs:20623) と候補ごと (solver.rs:20738) に sb > maxsb で止める |

- **当方の既定 0.125·x + 25000 は master の形と偶然同じだが、計測相手の 10.0.2 の 1/4。しかも両方の SCIP は根の強分枝の反復を予算の外に置き、当方は中に入れる** [確認]。
  根の強分枝が重い問題 (dano3_5 は根の強分枝だけで数万反復) では、当方の木の予算はほとんど残らない [推測、ログで sb_iters の根の分を見れば確かめられる]。
- 当方の `ENOMOTO_MIP_SB_BUDGET_HIGHS` (0.5·x + 100000、solver.rs:20570-20576) は 10.0.2 とほぼ同じ大きさだが、根の強分枝を除かない。
- 記録 (analysis/sbdegen_20261126 3 節) で「SCIP: 0.5 倍 + 100000」と書いたのは 10.0.2 には正しく、master には当たらない。

---

## 2. 強分枝の子の伝播と締まった境界 (probingbounds)

| 項目 | SCIP | 当方 | 違い |
|---|---|---|---|
| 子の伝播 | `maxproprounds` −2 (= propagating/maxrounds の設定、既定 100) で**既定で有効** (relps:98、1377-1378、scip_var.c:4184-4250)。全部の伝播器 (linear・pseudoobj・vbounds・衝突など) を probing でかける | 既定オフ。`ENOMOTO_MIP_SB_PROP=1`・`SB_PROBEBD=1` で有効 (solver.rs:20948-21007)。かけるのは行・含意 (`dom.propagate`)・目的 (`propagate_objective_sb`)・衝突の組 (`lit_prop`) | 既定が違う |
| 子が伝播か LP で実行不能 | 反対の向きに固定 (bdchg に積む)。両方なら刈る (relps:2054-2077)。衝突解析もする (downconflict) | 既定 (LP の実行不能・打ち切り値超え) で同じ: 片側なら固定、両側なら刈る (solver.rs:21341-21368)。伝播の矛盾は試作のときだけ | 既定で拾える実行不能が、伝播なしの分だけ少ない |
| 両方の子の境界の和 | `probingbounds` **TRUE** (relps:100)。両方の子の最終的な定義域の和 (下限は min、上限は max) をノードに置く (scip_var.c:4129-4153 付近の newlbs/newubs、relps:2025-2050)。片方の子が切れたら、もう片方の子の定義域をそのまま使う (`firstchild || cutoff` を渡す、scip_var.c:4351 付近) | 試作 `SB_PROBEBD=1` で同じことをする (solver.rs:20767-20771、21370-21396)。片方が切れた候補は列の固定だけで、残った子の締まった境界は置かない | 片側が切れたときの扱いが違う (SCIP は残った子で締まった境界も全部置く) [確認] |
| 置いた後 | 境界の変更があれば `SCIP_REDUCEDDOM` を返し、ノードの LP を解き直して分枝をやり直す (relps:2241-2251)。境界の変更 + 衝突が `maxbdchgs` 5 に達したら強分枝の繰り返しを止める (relps:97、1027) | 同じ: `bound_changes` が 5 で `Resolve`、終わりに正なら `Resolve` (solver.rs:21544、21587-21589) | 同じ |
| やり直したノードでの再利用 | 同じノードで強分枝した列は強分枝せず、前回の値で点数を付ける (`SCIPgetVarStrongbranchNode == nodenum`、relps:1495-1520、1610-1650) | 再利用の仕組みはない (`pre`・`parsb_spec` はその呼び出しの中だけ)。Resolve の後は信頼できない候補をまた強分枝する | 違う [確認]。当方は Resolve のたびに予算を使い直す |
| 子の LP の値でノードの下界 | 両側 valid な候補ごとに min(down, up) の最大をノードの下界にする (`provedbound`、relps:2016-2022、2225-2230) | 選んだ列の子の値を子の下界にするだけ (`last_sb`、solver.rs:17297、21598) | 小さい違い |

---

## 3. 点数の混ぜ方

### 3.1 式

SCIP (relps:632-680 `calcScore`):

```
dyn = (実行不能の葉 + 1)/(目的値で切った葉 + 1) × degeneracyfactor        (dynamicweights TRUE、relps:118)
score = dyn·( 0.01·s(conf, avgconf) + 0·s(conflen,·) + 1e-4·s(infer, avginfer) + 1e-4·s(cutoff, avgcutoff)
              + 0·gmiavg + 1e-5·gmilast )
        + 1.0/dyn · s(pscost, avgpscost) + 0.1·nlscore
min(f, 1−f) < 10·feastol なら score ×= 1e-6
```

- 各得点は SCIPbranchGetScore の積: max(下, 1e-6)·max(上, 1e-6) (branch.c:2236-2290、scorefunc 'p'・scorefac 0.167 は 's' のときだけ、set.c:85-86、sumepsilon 1e-6 は def.h:164)。
- 平均 (分母) はすべて max(·, 0.1) (relps:1358-1369):
  - avgpscost = 積 (全体の擬似費用(−0.5), (+0.5)) (scip_solvingstats.c:2123-2135)
  - avgconf = 積 (全体の VSIDS / (列数 × vsidsweight)) (同 2251-2266)。VSIDS は衝突ごとに vsidsweight /= 0.98 (conf/scorefac、set.c 158、conflict_graphanalysis.c:3499-3514)
  - avginfer = 積 (全体の推論の平均 / 分枝) (同 2387-2398)、avgcutoff = 積 (全体の打ち切りの平均 / 分枝) (同 2473-2484)
- 重みの既定: conflict 0.01、conflength 0、inference 1e-4、cutoff 1e-4、gmiavg 0、gmilast 1e-5、pscost 1.0、nlscore 0.1 (relps:75-82)。MIP では nlscore は 0。
- 信頼できない候補 (強分枝の前): 擬似費用の項は全体の平均で埋めた値で点数を付け、その点数の順に initcands に並べる (relps:1673-1706)。
  強分枝した候補は、擬似費用の項を強分枝の伸びの積に替えて同じ式で点数を付け直す (relps:2088-2131)。
- **最後の選び方** (relps:2190-2220): 「信頼できる候補 (と強分枝しない候補) の最良 bestps」が「強分枝しなかった信頼できない候補の事前の最良 bestuninit」より上で、かつ強分枝の最良より上のときだけ bestps。
  それ以外は強分枝の最良。**強分枝が 1 本でもあれば、強分枝しなかった信頼できない候補は選ばれない** [確認]。
- treemodel は既定オフ (treemodel.c:72)。

当方 (pseudocost.rs:407-455 `hybrid`):

```
dynw = (実行不能の葉 + 1)/(目的値で切った葉 + 1)   (ENOMOTO_MIP_GAP1 に 'w' があれば 1。既定 'u' なので有効)
score = dynw·( 1e-4·s(infer, max(平均², 0.1)) + 1e-4·s(cutrate, 0.1) + wc·s(conf, max(conf_avg², 0.1)) )
        + s(ps, max(avg_down·avg_up·0.25, 0.1)) / (dynw·degen_fac)
```

- 推論 = その列・向きの「分枝の後の行の伝播で締まった境界の数」の平均 (pseudocost.rs:391、記録は solver.rs:15290。全部の伝播で数えるのは試作 `ENOMOTO_MIP_INF_ALL`)。
- 打ち切り = c/(c + n + 1) (n は擬似費用の観測の数、pseudocost.rs:398-401)。分母の平均は固定 0.01 → max で 0.1。
- 衝突の重み wc = max(`ENOMOTO_T_MIP_CONFLICT_WEIGHT` 既定 0, cw_dyn·cw_mult, cw_floor) (pseudocost.rs:424-430)。
  既定は **`ENOMOTO_MIP_CONFLICT_WEIGHT_RAMP` = 0.1**: 200 ノードの窓ごとの「衝突 / ノード」の移動平均 r̄ から 0.1·σ((r̄ − 0.69)/0.1) (solver.rs:15160-15177、32236-32242)。最初の窓が終わるまで 0。
  衝突スコアは VSIDS と同じ形 (増分を衝突ごとに 1.02 倍、pseudocost.rs:199-235)。
- degen_fac は試作 `ENOMOTO_MIP_S4_DEGSCORE` のときだけ 1 以外 (solver.rs:20441-20456)。
- 信頼できない候補の事前の点数: 同じ式で、擬似費用は全体の平均と混ぜた値 (solver.rs:20430-20440、20470)。全候補をこの点数の降順に 1 本の列に並べる (solver.rs:20531)。
- 強分枝した候補: 伸びで同じ式 (solver.rs:21550)。
- **最後の選び方**: 候補を順に見て、強分枝しなかった候補 (信頼できる・予算切れ・lookahead で止まった・候補の上限 のどれでも) は事前の点数で最良と比べる (solver.rs:20738-20743)。
  **lookahead で止まった後の、強分枝していない信頼できない候補も、事前の点数 (全体の平均で埋めたもの) で強分枝の最良に勝てる** [確認]。

### 3.2 表

| 項目 | SCIP | 当方 | 違い |
|---|---|---|---|
| 形 (dyn × その他 + 擬似費用/dyn) | relps:666-674 | pseudocost.rs:455 | 同じ |
| 衝突の重み | 0.01、始めから (relps:75) | RAMP 0〜0.1、最初の 200 ノードは 0 (solver.rs:15094-15177) | **違う** (試した: 5 節) |
| 推論・打ち切りの重み | 1e-4・1e-4 | 1e-4・1e-4 (`ENOMOTO_T_MIP_HY_INF_W`・`_CUT_W`) | 同じ |
| 推論の数え方 | 分枝の後の全部の伝播 (衝突制約も) | 行の伝播だけ (既定) | 違う (小) |
| 打ち切りの正規化 | 全体の平均 / 分枝 (max 0.1) | 固定 0.1 | 違う (小) |
| 擬似費用の正規化 | max(avg, 0.1) | max(avg, 0.1) (`ENOMOTO_T_MIP_HY_PS_FLOOR`) | 同じ |
| 退化の係数 | degeneracyaware 1 (根以外): deg ≥ 0.8 で 10^(10(deg−0.7))、比 ≥ 2 で ×10·比。dyn に掛ける (= 擬似費用の項は割り、ほかの項は掛ける) (relps:1327-1351) | 既定は掛けない (S4_DEGSCORE で擬似費用の項だけ割る) | 違うが、係数 ≥ 10 なら両者とも強分枝しないので、対象の 3 問 (deg = 0) では効かない |
| gmilast 1e-5 | あり (relps:80) | なし | 小 |
| ほぼ整数の候補 | ×1e-6 (relps:677-678) | なし | 小 |
| **強分枝しなかった信頼できない候補を選べるか** | 強分枝が 1 本でもあれば選ばない (relps:2196-2215) | 選べる (solver.rs:20738-20743) | **違う [確認]、未試行** |

---

## 4. 強分枝の反復の上限・lookahead

| 項目 | SCIP | 当方 | 違い |
|---|---|---|---|
| 全体の予算 | 1.3 節 | 1.3 節 | 10.0.2 の 1/4、根の分を含む |
| 子 1 本の反復の上限 | inititer 0 (自動) = 2 × (双対の解き直しの LP の平均反復) × (1 + 20/ノード番号)、10〜500 (relps:1743-1770、maxinititer 500) | 4 × `avg_node_iters` (ノードの LP の反復 / ノードの LP の数。カットの後の解き直しも含む) × (1 + 20/ノード数)、10〜500 (`ENOMOTO_T_MIP_SB_ITERMULT` 4、solver.rs:20653、19893) | 当方のほうが緩い (倍率 2 倍、平均も大きめ) [推測: 平均の中身の差] |
| 候補の上限 | initcand 100 (relps:93、1386) | `ENOMOTO_T_MIP_SB_MAXCAND` 100 (solver.rs:20658) | 同じ |
| lookahead | 9 × (1 + 未初期化の候補 / 候補)。未初期化 = 観測 < 0.1 (relps:1742、1668-1669) | 9 × (1 + **信頼できない**候補 / 候補) (`ENOMOTO_T_MIP_SB_LOOKAHEAD`、solver.rs:20656-20657) | 当方のほうが大きくなりやすい |
| lookahead の数え方 | 最良を更新 → 0、同点 (SumGE) → +0.5、ほか +1 (relps:2133-2155) | 同じ (solver.rs:21555-21565) | 同じ |
| 打ち切り | candidx < ninitcands かつ lookahead < max かつ 境界の変更 + 衝突 < 5 かつ (candidx < maxlookahead または 予算内) (relps:1014-1029) | 候補ごとに 信頼できる/予算/上限/lookahead/時間 を見る (solver.rs:20738) | ほぼ同じ (SCIP は先頭の maxlookahead 本は予算を超えても続ける) |
| 強分枝を始める深さの制限 | なし (BRANCHRULE_MAXDEPTH −1、relps:72) | なし (`ROOTSB` は根を強める試作) | 同じ |
| 退化で強分枝しない | 係数 ≥ 10 (= deg ≥ 0.8 か比 ≥ 2)、根以外 (relps:1407) | deg ≥ 0.8 か比 ≥ 2、根以外 (solver.rs:20601-20606) | 同じ (度合いの測り方は簡略版、solver.rs:19938-19960) |
| LP 解が基底でない | 強分枝しない (relps:1387-1388) | 判定なし | 小 |
| 強分枝の子が整数解 | 確かめ、丸めも試す (checksbsol・roundsbsol TRUE、set.c:100-101) | 整数なら暫定解の候補 (solver.rs:21290 付近) | 小 |

---

## 5. ほかに目立つもの (手短に)

- filtercandssym FALSE・transsympscost FALSE (relps:123-124): 既定では対称性で候補を絞らない。当方も絞らない (下の子で軌道に沿って 0 にする `ENOMOTO_MIP_SB_OF` は別物)。
- dynamiclookahead FALSE (relps:113): 既定オフ。当方の `ENOMOTO_MIP_PLSB` (試作) が近い。
- degeneracyaware 1 (relps:120): 3 節の表のとおり。
- startrandseed 5・randinitorder FALSE (relps:112、116): 既定では乱数を使わない。当方は racing のときだけ ±10% (solver.rs:20520-20529)。
- usesmallweightsitlim FALSE、usesblocalinfo FALSE、storesemiinitcosts FALSE、discountfactor 0.2 だが collectancpscost FALSE (set.c:96) なので割引の擬似費用は使わない。
- confidencelevel 2 (relps:109): skipbadinitcands の検定にだけ効く (1.2 節)。

---

## 6. 試したものとの関係 (記録から)

| 試したもの | 何をしたか | 結果 (記録) | SCIP との残りの違い |
|---|---|---|---|
| `SB_PROBEBD=1` (treegap_20261123 7・9 節) | 子の伝播 + 両方の子の和 | swath1 2.6 倍少ないノード、dano3_5 1 割、mcsched 置いた境界 0 で変わらず、30n20b8 時間切れ | 片側が切れたときに残った子の境界を置かない。予算・lookahead は既定のまま (子の伝播で予算の使い方が変わる) |
| `SB_ADAPT=1` (sbadapt_20261124) | 予算・退化・子の伝播・衝突の床を特徴のシグモイドで | mcsched 時間切れ → 171 秒など。P の 4 スレッドで外れ。採らず | 部品の束ね方の問題で、各部品は SCIP と同じ方向 |
| `CONFLICT_WEIGHT=0.01` (treegap_20261123 5 節) | 衝突の重みの床を始めから 0.01 | mcsched 303 秒、swath1 65 → 46 秒、30n20b8 時間切れ | SCIP と同じ形になる。残りの違いは衝突の中身 (どの衝突が入るか) [推測] |
| `SBDEGEN=1` (sbdegen_20261126) | 予算を既定と 0.5·x + 100000 の間で内挿 | 3 問では退化で省くことが 0 回。予算を増やすとノード 0.54〜0.58 倍。P の 1 スレッド中立、4 スレッド 3% 遅く、採らず | 根の強分枝の分を除かない。skipbadinitcands がないので、増やした予算を観測 2〜4 回の候補にも使う |
| `MINREL=0` を「強分枝なし」とした比べ (treegap_20261121・20261123 3 節) | — | 「当方の強分枝は木を小さくしていない」(1.0〜1.4 倍) | **注意 [確認]: 今の `select_branch_inner` は `pc.min_reliable` を信頼度に使わない** (solver.rs:20627-20628 のコメント、計測時の 9845e3366 でも同じ)。`ENOMOTO_T_MIP_MINREL=0` は擬似費用と全体の平均の混ぜ方 (pseudocost.rs:285 の w を 1 に) を変えるだけで、強分枝は止まっていない。強分枝を止めるには `ENOMOTO_T_MIP_SB_MAXCAND=0` か `ENOMOTO_T_MIP_SB_LOOKAHEAD=0` (solver.rs:20738 で即座に打ち切り) が要る。3 節の結論は測り直しが要る |

---

## 7. 「根の下界は並ぶのに木が数倍大きい」(mcsched・swath1・dano3_5) に効きそうな違い (優先順)

前提の事実: 3 問とも退化で強分枝を省くことは 0 回 (sbdegen 2 節)。SCIP は relpscost → pscost で 5 倍以上ノードが増える (treegap_20261123 2 節)。当方は予算を HiGHS 並みにするだけで swath1・mcsched のノードが 0.54〜0.58 倍 (sbdegen 3 節)。mcsched は分枝の選択 11034 回に強分枝した候補が約 1442 (sbdegen 2 節)。

### 1. 強分枝しなかった信頼できない候補を選ばない (SCIP の bestuninit の規則) — 未試行、手間が小さい

- 違い [確認]: SCIP は強分枝が 1 本でもあれば、強分枝していない信頼できない候補は選ばない (relps:2190-2215)。当方は lookahead・候補の上限で止まった後の信頼できない候補を、全体の平均で埋めた事前の点数で選べる (solver.rs:20738-20743)。
- 効きそうな理由 [推測]: 強分枝で測った伸びは事前の見積もりより小さくなりがち (平均で埋めた値は「平均的な列」の値) なので、lookahead が止まった直後の候補 (事前の点数が次に高いもの) が強分枝の最良に勝ちやすい。
  その場合、当方は測っていない列で分枝し、木の上のほうの選びが擬似費用の平均に近いものになる。予算が小さい当方では lookahead で止まるノードが多いので、効き目が大きい側に出るはず。
- 試したものとの違い: SB_PROBEBD・SB_ADAPT・CONFLICT_WEIGHT・SBDEGEN はどれもこの規則に触れていない。
- 先に測ること: 強分枝をしたノードで、選んだ列が「信頼できず強分枝していない列」だった割合。今の数え (`poly_diag[7]`、solver.rs:20405-20407) は折れ線の診断の中だけなので、一般には数えていない。`SBTRACE` の行 (solver.rs:20319-20328) に足すのが手軽。

### 2. 予算を SCIP 10.0.2 の形にする: 0.5·x + 100000、**根の強分枝の反復を除く**、閾値 (prio) もこの予算で — SBDEGEN・SB_BUDGET_HIGHS との違いは根の扱い

- 違い [確認]: 1.3 節。10.0.2 は当方の 4 倍の大きさで、根の強分枝の反復は予算の外。prio (信頼度の閾値 1〜5) も 4 倍の quot で計算されるので、閾値 5 が長く続く。
- 根拠 (事実): SB_BUDGET_HIGHS でノード 0.54〜0.58 倍 (swath1・mcsched)、dano3_5 は 120 秒で 89 → 26 ノード。
- 試したものとの違い: SB_BUDGET_HIGHS・SBDEGEN は根の強分枝を予算に数えたまま。dano3_5 のように根の強分枝が重い問題では、木の予算の残りが違う [推測、根での `sb_iters` を見て確かめる]。
  P の 4 スレッドで遅れた (sbdegen 5 節) のは強分枝の手間そのもので、3 と組むと減らせる [推測]。

### 3. skipbadinitcands に当たる「観測 2 回以上の候補は、強分枝の最良が出た後は強分枝しない」 — 未試行

- 違い [確認]: 1.2 節。SCIP は既定で有効。目的値が正の 3 問では、式の上ではほぼ「観測 0〜1 回の候補にだけ強分枝を使う」になる [推測: 閾値が子の LP の値そのもののため]。
- 効きそうな理由 [推測]: 同じ予算で、まだ何も分かっていない列を多く測れる (情報の多い所に予算を使う)。2 の予算増と組むと、予算の増分が観測 2〜4 回の候補の測り直しに流れるのを防ぐ。4 スレッドでの手間の増えも抑えられる。
- 実装の注意: SCIP と同じに「子の LP の値」を閾値にするか、「伸び」にするかで効き方が大きく違う。先に SCIP の形 (絶対値) で寄せ、次に伸びでも見る。

### 4. 反復上限で止まった強分枝の子も観測に数える + Resolve の後に同じノードの強分枝の結果を使い回す

- 違い [確認]: SCIP は上限で止まった子の伸び (双対単体法の途中の値 = 下界) で擬似費用を更新し、回数も数える (relps:1979-2014、WITH_LPSOLSTAT 未定義)。当方は数えない (solver.rs:21311-21316)。
  SCIP は REDUCEDDOM の後の同じノードで強分枝の値を使い回す (relps:1610-1650)。当方は Resolve のたびに強分枝し直す。
- 効きそうな理由 [推測]: どちらも予算の空費を減らし、2・3 と同じ方向 (情報の多い所に予算) に効く。効き目は 1〜3 より小さいと見る。上限で止まる割合 (`sb_capped`) と Resolve の回数を先に数えるとよい。
- 試したもの: `ENOMOTO_MIP_PCL=a` は「今の擬似費用より大きいときだけ」入れる形で、SCIP (常に入れる) とは違う。

### 5. 子の伝播の範囲を SCIP に寄せる (片側が切れたら残った子の境界も置く) — SB_PROBEBD の続き

- 違い [確認]: 2 節。SCIP は片側が切れた候補でも、残った子の伝播の結果の境界を全部ノードに置く (scip_var.c の firstchild || cutoff)。当方の SB_PROBEBD は列の固定だけ。
  SCIP の relpscost の DomReds は mcsched 7659・dano3_5 492 (treegap_20261123 4 節) なのに、当方の SB_PROBEBD の置いた境界は mcsched 0・dano3_5 17 (同 7 節)。この差の一部がここから来る [推測]。
- 試したものとの違い: SB_PROBEBD (両方の子が残ったときだけ和を置く)。30n20b8 の害は「根のすぐ下で正しい強い推論が入って道が変わる」もの (sbadapt 5.3 節) で、この部品でも出うる。

### 入れなかったもの

- 衝突の重み 0.01 を始めから: SCIP と同じ形にすると mcsched・swath1 に効くが 30n20b8 を壊す (試した)。残りの違いは衝突の中身で、点数の付け方ではない [推測]。
- 退化の係数 (DEGSCORE・SB_NO_DEGEN_SKIP・SBDEGEN): 3 問では退化で省くことが 0 回なので効かない (事実)。
- 擬似費用の単位の作り方 (pseudocostdelta 1e-4・距離の下限 0.1・全体の平均と混ぜる w): 違いはあるが、3 問は目的値の伸びがはっきり出る問題で、効き目は小さいと見る [推測]。

### 先に要る確かめ (どれもコードを変えずにログか既存の診断で)

1. 6 節の注意: `ENOMOTO_T_MIP_SB_MAXCAND=0` で「強分枝なし」を測り直し、当方の強分枝がどれだけ木を小さくしているかを SCIP の 5 倍と比べる。
2. 1 の割合 (選んだ列が強分枝していない信頼できない列だった割合)。
3. 根の強分枝の反復が木の予算 (0.125·x + 25000) に占める割合 (dano3_5・mcsched・swath1)。
