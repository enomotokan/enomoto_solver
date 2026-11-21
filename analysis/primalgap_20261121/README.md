# primalgap_20261121: 時間切れのときの暫定解が SCIP より大きく悪い問題の切り分け

診断だけ (既定は変えない、試作なし)。

- 当方: ビルド 70a0b6064 (`pip install -e .`)、4 スレッド既定、600 秒、種 1〜4 (`ENOMOTO_T_MIP_SEED`)、`ENOMOTO_MIP_LOG=1`。暫定解の出どころはログの `new incumbent ... [ラベル]`。
- SCIP: pyscipopt 6.3.0 (SCIP 10.0.2)、1 スレッド、600 秒、`randomization/randomseedshift` = 種 1〜2。出どころは BESTSOLFOUND の時刻・値とログ行頭の文字 (L = LNS 系 (rens/rins/alns)、o = objpscostdiving、d = その他のダイビング、* = 木の LP 解、r = rounding、p = 自明系 (trivial/clique/vbounds)、i = oneopt 等)、および統計の Primal Heuristics 表。
- 実行: `python run.py O` (当方)、`python run.py S` (SCIP)。結果 `res_O.jsonl` / `res_S.jsonl` (各行の `result.hist` が暫定解の推移)、ログは `logs/*.log.gz`、SCIP 統計は `logs/*_scip_s*.stat.gz`。
- O は 2 時間制限で 1 回止まり、途中結果を push して続きから再開した (sorrell3 種 3 をやり直し)。S も同様 (kakapo 種 2 をやり直し)。

## 1. 最終値

| 問題 | 当方 種1 | 種2 | 種3 | 種4 | SCIP 種1 | SCIP 種2 | 当方の下界 | SCIP の下界 |
|---|---|---|---|---|---|---|---|---|
| brazil3 | 231 | 243 | 240 | 225 | 80 | **解なし** | 20 | 20.2 / 15.3 |
| momentum1 | 212309 | 341174 | 411921 | 527463 | 115639 | 115673 | 96259〜96278 | 96348 / 96317 |
| kakapo | 4788 | 4536 | 4941 | 4545 | 3672 | 4500 | 190〜325 | 1209 / 1195 |
| sorrell3 | -14 | -13 | -13 | -14 | -16 | -15 | -19 | -19.61 |

ノード数: 当方 (4 スレッド合計) brazil3 4280〜5577、momentum1 370〜621、kakapo 47015〜112405、sorrell3 101〜105。SCIP brazil3 77/86、momentum1 1152/1436、kakapo 76471/75857、sorrell3 1/1。

## 2. 推移と出どころ

### brazil3
- 当方: 2〜5 秒に Local-MIP が 300 台の解を出し、Local-MIP が 1 ずつ 20〜30 回改善 (合計 -38〜-50)、28〜48 秒に RINS が 1 回で -27〜-90 (231/246/240/225)。その後 560 秒近く改善なし (種 2 だけ 467 秒に tree crossover で -3)。
- SCIP 種 1: 360 秒に objpscostdiving が 1 回 (45.65 秒) で 80 を出したのが唯一の解。ダイビング LP 458850 反復 = LP 反復全体の約 41%。
- SCIP 種 2: 600 秒で解なし (ダイビング LP 72200 反復 = 6%、objpscostdiving は 0 回)。
- 当方のダイビング: 主スレッドの状況行で dive_iters 8967 / lp_iters 1421307 (0.6%)。ダイビングから暫定解は 0 回。

### momentum1
- 当方: 9〜16 秒に trivial 533875、33〜39 秒に vbounds (loose) の sub-MIP で 527463。そこから **456 秒 (種 1)、578〜581 秒 (種 2・3) まで改善なし、種 4 は最後まで 527463**。改善はすべて木の local branching / proximity (他スレッド) で、始まった時刻の差がそのまま最終値の差 (212309〜527463)。
- 当方の根の RENS: SUBFIX RENS が 1016 自由整数列で 6 秒時間切れ (0 ノード)、続く既定 RENS の並列 sub-MIP は 0.02 秒で Infeasible。locks / clique / vbounds (tight) / root reduced cost の並列 sub-MIP も 0.01〜0.02 秒で Infeasible (4 種とも同じ)。
- 当方の木: 456 秒でノード 83 (4 スレッド)。強分枝 119 秒。ノード 1 つに数秒。
- SCIP 種 1: 5 秒 p (vbounds) 353054 → **37 秒 L (rens 1 回、15.9 秒) 122054** → 234 秒 L (rins) 115639。
- SCIP 種 2: 6 秒 p 353054 → 22〜98 秒 L (alns) 346636〜333788 → 136〜387 秒 L (alns 6 回 best、rins 2、crossover 1) で 199035 → 115673。
- SCIP は 2 種とも LNS (L) だけで 11.6 万に届く。当方は根の LNS が全部 0.02 秒で「実行不能」になり、木の LNS が回るまで 450 秒以上かかる。

### neos-5107597-kakapo
- 当方: 1〜2 秒 Local-MIP で 23328〜30501 (種 3・4 は find thread で 24813/26775)。その後 **83〜493 秒までほぼ横ばい (2.3〜2.7 万)**、224 秒 (種 1)、271 秒 (種 2)、442 秒 (種 3)、493 秒 (種 4) に急落して 5000 前後へ。改善の回数は tree proximity 64〜109 回、tree rounding 17〜98 回、ほか mutation/DINS/crossover 少し。ダイビングからは 0 回。
- 当方のダイビング: dive_iters / lp_iters = 5645 / 2032331 (0.28%、種 1)、4723 / 3204771 (0.15%、種 4)。
- SCIP 種 1: 0.6 秒 o 145.7 万 → 26 秒 L 57348 → 29〜121 秒 **o (objpscostdiving) で 20295 → 4257** → 91〜133 秒 d/* で 3897 → 291〜324 秒 */d/r で 3672。統計: objpscostdiving 45 回で 26 解 (best 7)、LP solutions 26 (best 26)、pscostdiving 5、feaspump 5。ダイビング LP 411450 / 8576099 (4.8%)。
- SCIP 種 2: 0.9〜21 秒 o で 16740 まで、47〜50 秒 d、101〜144 秒 */r で 4500。統計: objpscostdiving 69 回 34 解、rootsoldiving 21 解 (best 2)、adaptivediving 3、linesearchdiving 2、LP solutions 10。ダイビング LP 794795 / 7841736 (10%)。
- SCIP は 2 万台を 30 秒以内に抜け、5000 台に 100 秒前後で着く。当方は同じ水準に 224〜493 秒かかる。

### sorrell3
- 当方: 0 秒に tree (VB01 の sub-MIP) -13、種 1・4 は 51 秒に Local-MIP -14。それ以外は改善なし。根の LP が 46〜48 秒、根の切除平面 (前景) が 149 秒、背景の切除平面が 374 秒まで続き、木の開始は 374 秒。
- 当方の根の RENS: 149 秒時点の LP (-20.19) で SUBFIX RENS 400 自由列 9 秒時間切れ、既定 RENS 372 自由列の並列 sub-MIP は 89〜131 ノード・91〜130 秒で -7 止まり (「RENS (more nodes) skipped」)。背景の切除平面で LP が -19.91 になった後に RENS をやり直していない。
- SCIP: 3 秒 p (clique/vbounds/trivial) -10/-11、**RENS 1 回 (146 秒 / 110 秒) が 591 秒 / 493 秒に -16 / -15**。種 1 は 252 秒に oneopt 等で -13。SCIP は 600 秒間ずっと根 (ノード 1) で、RENS は切除平面を 381 本足した後の LP (-19.6) で走っている。

## 3. 運か弱点か

| 問題 | 当方の揺れ | SCIP との差 | 判定 |
|---|---|---|---|
| brazil3 | 225〜243 (幅 8%) | SCIP は 80 / 解なし | **SCIP 側の当たり外れが大きい。** 11/17 の 80 は objpscostdiving 1 回の当たり。当方は毎回 230 前後で安定しているが、80 や CM2 pe の 25 があるので「良い解は届く所にある」。弱点としてはダイビングの量 (0.6% vs SCIP 6〜41%) |
| momentum1 | 21.2 万〜52.7 万 (2.5 倍) | SCIP は 2 種とも 11.6 万 | **弱点。** 当方の揺れ自体は「木の LNS が何秒に初めて当たるか」の差で、最良の種でも SCIP の 1.84 倍。原因は根の LNS が 0.02 秒で Infeasible になること |
| kakapo | 4536〜4941 (幅 9%) | SCIP 3672 / 4500 | **弱点 (中程度)。** 当方の 4 種すべてが SCIP の悪い方 (4500) 以上。差の大半は「2 万台から抜けるまでの 200〜500 秒」 |
| sorrell3 | -13〜-14 | SCIP -16 / -15 | **小さな弱点。** SCIP は 2 種とも切除平面後の根の RENS 1 回で -15/-16。当方は RENS を切除平面前の LP で 1 回だけ回している |

## 4. SCIP で効いている手は当方にあるか・回っているか

事実と推測を分けて書く。

1. **LNS (RENS/RINS/ALNS) を根で効かせる — momentum1**
   - 事実: 当方の根の RENS / locks / clique / vbounds (tight) / root reduced cost の並列 sub-MIP が 4 種とも 0.01〜0.02 秒で Infeasible。SCIP の rens は 15.9 秒で 122054 を出している。当方の vbounds (loose) の sub-MIP (1932 自由列) は 527463 を出せている。
   - 推測: 固定の作り方 (LP 解の丸めの許容誤差、または係数幅 7.9e9 のせいで固定後の presolve が誤って実行不能と判定している) に問題がある可能性が高い。0.02 秒の実行不能はノードを 1 つも解いていないので、sub-MIP の presolve で落ちていると見られる。
2. **目的を緩めたダイビング (objpscostdiving 相当) — kakapo, brazil3**
   - 事実: 当方の `DiveKind` に objpscostdiving に当たるもの (目的関数を外し、擬費用で可能解へ向かうダイビング) はない。当方のダイビングは深さ 10 ごとに 3/7 の 2 回、反復の上限は `0.05 × (成功+1)/(呼出+1) × ノード反復 + 1000` で、成功しないと上限が縮む。実際の dive_iters は LP 反復の 0.15〜0.6%。SCIP はダイビング LP が 4.8〜10% (kakapo)、6〜41% (brazil3)。kakapo で SCIP の 2 万台→4000 台の主役は objpscostdiving (26/34 解)。
   - 推測: 当方は「暫定解は悪いが可能解はある」状態で、可能解を作るダイビングが回らず、proximity 頼みになっている。
3. **根の切除平面の後に RENS をもう一度 — sorrell3**
   - 事実: 当方の RENS は前景の切除平面を終えた時点の LP (-20.19) で 1 回だけ。背景の切除平面で -19.91 になった後には回していない。SCIP の rens は 381 本の切除平面後の LP で 1 回走り、-16 / -15 を出した。当方の RENS sub-MIP は 1 ノード約 1 秒と遅い (16.9 万の衝突行)。
   - 推測: 切除平面後の LP は整数に近い列が多く、固定率が上がって sub-MIP が小さくなる。
4. **木のノードの速さ — momentum1**
   - 事実: 当方 (4 スレッド) 370〜621 ノード、SCIP (1 スレッド) 1152〜1436 ノード。
   - 推測: 木の LNS の始まりが遅れる一因。ただし 1 の方が効くと見る。

## 5. 直すならどこか (優先順位つき)

1. **momentum1: 根の LNS の sub-MIP が 0.02 秒で Infeasible になる原因の調査** (RENS / locks / clique / vbounds (tight) / root reduced cost に共通)。固定に使う値と許容誤差、sub-MIP の presolve の実行不能判定を見る。SCIP の rens はここで 122054 (当方最良の 0.58 倍) を出しており、効果がいちばん大きく見込める。
2. **kakapo / brazil3: 目的を外したダイビング (objpscostdiving 相当) を足す、またはダイビングの反復上限を見直す。** 今の上限式は成功しないと縮み、LP 反復の 1% 未満しか使っていない。SCIP は各ダイビングに `maxlpiterquot` 0.05 前後を別々に与えている。
3. **sorrell3: 背景の切除平面が終わって LP が締まった後に RENS をもう一度回す** (固定率が上がっていれば)。差は 1〜3 と小さいので優先度は低め。
4. momentum1 の木のノードの遅さ (強分枝 119 秒、1 ノード数秒) は別件として記録だけ。

## 6. 直す案 1 の結果: momentum1 の根の sub-MIP が 0.01〜0.02 秒で Infeasible になる件

### 原因
- LP probing の後、presolve の集約で等式行 135 (`x54 = x1046+0.7x1048+x1050+x1055+x1057+2y`) を通して代入した結果、`-x54 + 2·y + 0.0274·z ≤ -2e-9` のような行ができる。伝播 (src/presolve/propagate.rs の `lb[k] = candidate`) がそこから連続列に 2e-9 / 1.7e-9 / 1.4e-9 といった下限を付ける。これは外側の presolve 済み問題 `self.p` の側で起きる。
- root reduced cost と locks の sub-MIP は、連続列を下限 (2e-9) に、2 値列 y を 0 に固定する。sub-MIP の presolve の propagate_split は行 `x54 - 5.98·y ≤ 0` の最小活動 2e-9 を `PROPAGATE_EPS` (1e-9)·(1+scale) と比べ、超えているので実行不能と判定する (0.003 秒で Presolved::Infeasible)。2e-9 は可能性の許容誤差 1e-6 よりはるかに小さく、誤判定である。sub-MIP を書き出して SCIP で解くと 115670 が出る。
- 既定の SUBMIP_HIGHS では sub-MIP の presolve に `submip=false` で入るので、判定は `params.submip` (solve_problem の入口) で切り替える。

### 直し (コミット d0a9eea36)
- sub-MIP を解いている間だけ、presolve の実行不能判定の許容誤差 `infeas_tol` を `max(従来値, crate::mip::domain::FEASTOL=1e-6)` にする。新しい閾値は置かず、既存の可能性の許容誤差を使う。許容誤差内の違反は forcing 行として扱う。sub-MIP の解は元問題で検査されてから受け取るので、緩めても誤った解は入らない。
- 既定で入。`ENOMOTO_MIP_SUBMIP_FEASTOL=0` で以前の動きに戻る。
- 診断用に `ENOMOTO_MIP_DEBUG_DUMP_SUBINF=<prefix>` を足した。sub-MIP の presolve が Infeasible を返したとき、その問題を `<prefix>_<n>.mps` に書き出す。
- 直した後も RENS (SUBFIX)・clique・vbounds (tight) は Infeasible で終わる。書き出した 4 問題を SCIP で解くと 4 問題とも infeasible だったので、これは本当の実行不能 (固定が強すぎる) であり、誤判定ではない。

### 確かめ
- fuzz (analysis/rootgap_20261121/fuzz.sh の組を `fuzz.sh` で既定のまま回した): tiny 0/400, highs 0/300, objint 0/1000, slack の食い違い 0, tiny2 0/400, objint2 0/1000, slack2 の食い違い 0。
- cargo test: 419 passed, 0 failed。

### momentum1 (4 スレッド、種 1〜4、600 秒): def (直しあり) と sft0 (`ENOMOTO_MIP_SUBMIP_FEASTOL=0`)
結果は res_F.jsonl、ログは logs/momentum1_{def,sft0}_s*.log.gz。「最初の改善」は vbounds (loose) の 527463 より良い解が最初に出た時刻と、その値・出どころ。

| 種 | def 最終 | def 最初の改善 | sft0 最終 | sft0 最初の改善 |
|---|---|---|---|---|
| 1 | 173337 | 43.3 秒 211815 (locks) | 527463 | なし |
| 2 | 154070 | 49.9 秒 237923 (root reduced cost) | 193178 | 399.5 秒 469735 (木の local branching) |
| 3 | 154136 | 50.9 秒 211815 (locks) | 218955 | 503.5 秒 469735 (木の local branching) |
| 4 | 154135 | 44.0 秒 211815 (locks) | 347586 | 545.0 秒 469735 (木の local branching) |

- 4 種とも def の方が良い。最初の改善は 400〜545 秒 (種 1 はなし) から 43〜51 秒に早まった。最終値も 19.3〜52.7 万から 15.4〜17.3 万になった。それでも SCIP (11.6 万) との差は残る。
- 根の sub-MIP が「0 ノードで Infeasible」で終わった数は、1 回の実行につき sft0 で 5、def で 3 (RENS・clique・vbounds (tight)、いずれも本当の実行不能)。

### 同じ型がほかの問題にあるか
- 以前のログ (B〜E の組) で数えると、0 ノードで Infeasible の sub-MIP は momentum1 で 8 回中 5 回、sorrell3 で 1 回 (clique)、kakapo で 1 回 (RENS) だった。後の 2 つが誤判定かどうかは未確認。
- P・M の広い計測で数える。台本は `wrun.py`、分担表は `shards_P.json` (6 組、348 回、各 0.47 時間) と `shards_M.json` (20 組、372 回、各 1.83〜1.85 時間)。def と sft0 を比べる。実行は `python analysis/primalgap_20261121/wrun.py P K` (K=0..5) と `python analysis/primalgap_20261121/wrun.py M K` (K=0..19)。d0a9eea36 を含むコミットからビルドすること。結果 res_SET_sK.jsonl の `submip` 欄に、sub-MIP の数、0 ノードで Infeasible の回数と時間、そのうち根で起きた分、ラベル別の内訳が入る。

### 回したビルド
- P 2: c3bdeaa7c (d0a9eea36・1cb61bc9a 以降) を `pip install -e .` で入れて回した (0.1.10)。58 行・crash 0。
- P 5: c3bdeaa7c (1cb61bc9a・d0a9eea36 を含む、リポジトリの根で `pip install -e .`、0.1.10)。58 行・crash 0。
- P 4: c3bdeaa7c (d0a9eea36・1cb61bc9a 以降、`pip install -e .`、0.1.10)。58 行・crash 0。
- P 1: c3bdeaa7c (d0a9eea36・1cb61bc9a 以降、`pip install -e .`、0.1.10)。58 行・crash 0。
- P 3: c3bdeaa7c (d0a9eea36・1cb61bc9a 以降、`pip install -e .`、0.1.10)。58 行・crash 0。
- P 0: c3bdeaa7c (d0a9eea36・1cb61bc9a 以降、`pip install -e .`、0.1.10)。58 行・crash 0。

## 7. 広い計測 P の結果 (6 分担そろい、ci_P.txt)

- `python ../rootgap_20261121/ci.py 60 sft0 def res_P_s*.jsonl`: def/sft0 (直しあり/なし) のずらし幾何平均の比 1.005 (95% [0.992, 1.018])、解けた sft0 137 / def 136。
- 全種で 1.25 倍超の遅れ: なし。optimal 同士の目的値の食い違い: なし。crash 0。
- 解けた数の差 1 は neos5 種 2 (sft0 は 42.8 秒で最適、def は時間切れで暫定解 15・下界 14.35)。他の種の neos5 は両方解けている。
- 1 回だけの遅速は両向きに散らばる (10teams は種 1・2 で def が遅く、種 3 で def が速い)。
- 0 ノードで Infeasible の sub-MIP は分担により増減し、根の分はほぼ変わらない。P では誤判定の起きる問題が少なく、momentum1 (M) のような効きは P では見えない。
- 解けた数が 1 減ったので、neos5 を種を増やして確かめる。
