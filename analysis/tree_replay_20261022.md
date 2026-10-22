# neos-2978193-inde: HiGHS の分枝をなぞって木の差を切り分ける (2026-10-22)

- 環境: Linux x86_64、CPU 4 (専用コンテナ、ソルバーは同時に 1 つだけ)、ブランチ `inv/tree-replay-20261022` (develop 7dc7e98 から)、
  highspy 1.15.1、HiGHS 1.15.1 を計装したもの (`analysis/tree_replay_20261022/highs_instrument_replay.patch`、`scripts/highs_instrument_tree.patch` を含む)
- 計測: 「1 スレッド」は ENOMOTO `ENOMOTO_MIP_THREADS=1` + `solve(..., 1)`、HiGHS `threads=1`。「打ち切り値つき」は ENOMOTO
  `ENOMOTO_MIP_CUTOFF=-2.38805`、HiGHS `objective_bound=-2.38805` (最適値 -2.388061686 の証明だけの比較)。「4 スレッド・60 秒」は打ち切り値なし。
- スクリプト・ログ: `analysis/tree_replay_20261022/` (`run_highs.sh`・`run_eno.sh`・`run_any.sh` 1 回の実行、`highs_to_replay.py` HiGHS のログ
  からなぞりのファイル、`replay_stats.py` なぞりの集計、`node_detail.py` 同じノードの固定を列ごとに並べる、`batch*.sh` 計測の並び、
  `replay_h1_cut_r.txt` なぞりのファイル、`highs_conflict_cuts.txt` HiGHS の衝突カット、`logs.tar.gz` 各回のログ)

## 結論

1. **原因は「ノードで下界を上げる力」の側、具体的には双対証明から作るカットとその伝播** (事実、§2〜§4)。HiGHS の分枝と
   ノードの順をそのままなぞらせても、既定の ENOMOTO の木は 4261〜17521 ノード (5 回、HiGHS 2468) で小さくならない。同じノードで
   ENOMOTO が固定する 0-1 列は HiGHS より約 19 本少ない (56 本中)。HiGHS の固定の大半は、葉の双対証明から作る y だけの
   「衝突カット」(`generateConflict`、木全体で 738 本・平均 19 列) をカットプールで伝播したものと、それに続く行の伝播。
2. ENOMOTO で同じカットが効かない理由は 3 つ (事実、§4)。
   (a) 証明から、境界が大域と同じままの**整数列**を右辺に移して消す。証明が「このノードで固定した列」だけの局所的なものになり、
   CMIR で強めても弱い。最初の葉で HiGHS は「y 8 本の和 >= 5」、ENOMOTO (前回の試作 SIDE_REDUCE を含む) は固定した 4 本の「>= 2」。
   (b) 前回の試作 `PROOF_CMIR_POOL` は CMIR のカットをカットプールに入れるはずだったが、**プールは根の密なカットで非零数の上限に
   達していて、1 本も入っていなかった**。(c) カットプールの伝播 (`ENOMOTO_MIP_CUTPOOL_PROP`) は既定では使わない。
3. 試作の組 **kpnf** = `ENOMOTO_MIP_LP_INT_BOUNDS=1 ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap
   ENOMOTO_MIP_CUTPOOL_PROP=1` で、1 スレッド・打ち切り値つきの木は 349・435 ノード (43〜116 秒。既定は 600 秒で 1 万ノード超でも終わらない、
   HiGHS 2503 ノード・40 秒)。**4 スレッド・60 秒 (打ち切り値なし) で 3 回とも最適: 59.2・40.2・44.2 秒** (既定は 3 回とも時間切れ)。
   必須なのは `PROOF_KEEP_INT` と `PROOF_CMIR_POOL=nocap` (+ `PROOF_CMIR`) で、この 3 つだけ (mink) でも 4 スレッド 3/3 (58.6・34.7・56.3 秒)。
4. 分枝の選び方の側は、上の試作の後では主因でない (推測寄りの事実)。試作の下では自前の分枝 (1145〜1625 ノード、kpnf 349・435) が
   HiGHS の分枝のなぞり (999 ノード) と同程度。既定の下での「分枝の側」の寄与は、なぞり自体が根の揺れで 4 倍ぶれるので測れなかった。
5. 副作用 (40 問、4 スレッド・60 秒): §6。

## 1. 揺れを除く (1 スレッド・打ち切り値つき)

| | ノード | 時間 | 決定的か |
|---|---|---|---|
| HiGHS (計装版、threads=1) | **2503** (評価したノード 2468: 分枝 1289、葉 1179) | 40〜42 秒 | 2 回で HP/HB 行まで完全に同じ。計装を足した後の 4 回も同じ木 |
| ENOMOTO 既定 | 10969 で時間切れ (600 秒、下界 -2.3911) | — | 決定的でない (下記) |

- ENOMOTO は 1 スレッドでも決定的にならない。根 (再スタート前) のカットと時刻で決まる判断が回ごとに違い、再スタートで残すカットが
  406〜523 本とぶれる (`ENOMOTO_MIP_NO_BG_CUTS` でも 406・420 本)。この揺れを止めることはしていない (§7)。
- HiGHS は再スタート後 (56 列の 0-1、8 列固定) で木を作る。ENOMOTO も 8 列を固定して再スタートする (同じ 8 列かは確かめていない)。

## 2. HiGHS の分枝をなぞる (`ENOMOTO_MIP_BRANCH_REPLAY`)

HiGHS に `HIGHS_PATH_LOG` (評価したノードごとに `HP 通し番号 深さ 結果 LP値 下界 行数 強分岐中 固定数 再スタート回数 経路...`、
分枝ごとに `HB 列 値 上か 退化係数 ...`、列は `postSolveStack.getOrigColIndex` で元の番号) を足し、`highs_to_replay.py` で
「経路 (分枝した列と向きの集合) → HiGHS の処理順・結果・分枝した列」のファイルにする。ENOMOTO の試作は木の主スレッド (1 スレッド) で:
- 待ち行列から取り出したノードの経路がファイルにあれば、同じ列で分枝する (強分岐・オービトープの列の置き換えはしない)。
- ノードは HiGHS の処理順で選ぶ (待ち行列の優先度 = seq、`NodeQueue::prio_only`)。次のノードが子・兄弟なら潜りとして続ける。
- ファイルにないノード (HiGHS が刈ったノードの下など) は自前の規則で分枝し、親の直後に処理する。
- `ENOMOTO_MIP_REPLAY_LOG` でノードごとに `ER` 行 (終わり方、LP 値、固定された整数列の数と理由の内訳、HiGHS の結果)。

結果 (1 スレッド・打ち切り値つき、なぞりのファイルは同じ):

| 設定 | ENOMOTO のノード | うち HiGHS のファイルにある | HiGHS の葉で ENOMOTO が分枝した数 | 同じノードの LP 値 (ENOMOTO − HiGHS) の平均 | 固定した整数列 (ENOMOTO − HiGHS) |
|---|---|---|---|---|---|
| 既定 (5 回) | 4261, 16257(*), 4675, 7141, 17521(*) | 962〜1085 | 202〜296 | −0.012〜+0.004 | −10.3〜−12.9 |
| 前回の試作 full (3 回) | 10569, 3941, 8339 | 666〜1080 | 139〜276 | −0.010〜+0.010 | −11.4〜−12.1 |
| HiGHS の衝突カット 738 本をプールへ (上限のため実は入らず) | 16099 | 1032 | 274 | | −10.3 |
| **kp** (下の §5、PROP_FIXPOINT 込み) | **999** | 429 | 47 | +0.006 | −5.0 |

(*) 900 秒で時間切れ。固定した整数列の差は、ENOMOTO が再スタートで固定した 8 列を数え HiGHS は数えない (再スタート後の問題から消える)
分、ENOMOTO 側が 8 多く出る。実際の差は表の値 − 8 (既定で約 −19 本)。

- 既定でも HiGHS の分枝をなぞれば自前の分枝 (1 万超) より小さい回がある (4261) が、4 倍ぶれて HiGHS (2468) に届かない。
  なぞった後の木の大きさは、ENOMOTO の各ノードの刈れ方 (下界の側) で決まっている。

## 3. 同じノードでの固定の内訳 (1 ノードあたりの整数列の境界の変更、ENOMOTO が LP まで進んだノードで平均)

| | 分枝 | 行 | クリーク | カットプール | 衝突プール | 被約費用 | 対称性 | 双対証明 | その他 |
|---|---|---|---|---|---|---|---|---|---|
| HiGHS | 13.7 | 12.2 | 4.6 | **8.2** | 1.8 | 0.3 | 0.4 (orbital) | (カットプールに含む) | U 1.0 |
| ENOMOTO 既定 | 13.3 | 1.9 | — | 0 | 0.5 (リテラル) | 2.2 | 2.2 (オービトープ) | 0.7 | |

(HiGHS: `localdom` の変更の記録の理由。`Reason::unspecified` を被約費用 (index 1)・対称性 (index 2) で分けた。ENOMOTO: 変更の記録の
理由の値。どちらも計装の診断のみで動作は変えない。)

- HiGHS のカットプールで固定に使われたカットを書き出すと (`HCUT`・`HADDC`)、**ほぼ全部が `generateConflict` (双対証明の CMIR) の
  y だけのカット** (固定に使われた 141 本中 139 本、長さ 4〜30。残り 2 本は分離器の generateCut)。行の固定 (12.2) の多くは、カットプールの固定の後に施設の数の行 (8 本、y 8 本の和 = 4 か 5)
  で残りが決まるもの。
- 最初の葉 (seq 1、y20746 = 0、LP が打ち切り値で止まる) で HiGHS は「y20744〜y20751 の和 >= 5」を作り、seq 3 でこのカットが 4 列を固定する。

## 4. ENOMOTO で同じことが起きない理由 (列ごとの比較、`node_detail.py`)

1. 証明の作り方: `add_proof_from` は、ノードの境界が大域の境界と同じ列を右辺に移す (既定)。整数列も移すので、残るのはそのノードで
   固定した列だけ。seq 1 の証明は y20746・y20749・y20750・y20751 (分枝とオービトープで固定) の 4 列で、CMIR で「和 >= 2」。
   `ENOMOTO_MIP_PROOF_KEEP_INT` (大域的に固定されていない整数列は残し、連続列だけを右辺に移す) にすると、同じ葉で HiGHS と同じ
   「y20744〜y20751 の和 >= 5」になる (次の葉では「>= 6」)。
2. カットプールの非零数の上限: 根の密なカット (平均 1000 列超) でプールは上限 (20 × 問題の非零数) に達していて、`add_to_pool` は
   黙って捨てる。前回の調査の `PROOF_CMIR_POOL` も、この調査の最初の「HiGHS のカットを渡す」実験 (16099 ノード) も、カットは 1 本も
   入っていなかった (`CPP` の診断で確認)。`ENOMOTO_MIP_PROOF_CMIR_POOL=nocap` は上限を見ずに入れる。
3. 伝播の順: カットプールの伝播 (`propagate_cut_pool`) はオービトープの固定より前に 1 回だけ。HiGHS の seq 3 の固定は、対称性で
   2 列固定した後にカットが効くもの。`ENOMOTO_MIP_PROP_FIXPOINT` は各段を不動点まで回す (ただし §5 では外した方が良かった)。
- 「答えを先に渡す」実験: HiGHS の衝突カット 738 本を (上限を見ずに) 木の前にプールへ入れ、カットプールの伝播と不動点を使うと、
  なぞりの木は **21 ノード** で終わる (`rpd_4`)。HiGHS が木全体で学んだカットを最初から持つので HiGHS より強いのは当然で、
  「足りないのはこのカット」の確認にだけ使う。
- 対称性: HiGHS の orbital fixing は ENOMOTO のオービトープ (8×3、8×4) が固定しない列 (y20750・y20782・y20743 など) も固定する。
  衝突プール (F) でも HiGHS は早くから固定する (seq 8 で 5 列)。この 2 つは試作では手を付けていない。

## 5. 試作の組と部品の切り分け (担当の問題)

1 スレッド・打ち切り値つき (300〜900 秒):

| 設定 | ノード (時間) |
|---|---|
| 既定 | 10969 で時間切れ (600 秒) |
| kp = LP_INT_BOUNDS + PROOF_KEEP_INT + PROOF_CMIR + PROOF_CMIR_POOL=nocap + CUTPOOL_PROP + PROP_FIXPOINT | 1625 (95 秒)、1145 (96 秒) |
| kp から LP_INT_BOUNDS を外す | 1837 (98 秒) |
| kp から PROOF_KEEP_INT を外す | 2439 (106 秒) |
| kp から PROOF_CMIR を外す | 3569 (151 秒) |
| kp から PROOF_CMIR_POOL を外す | **9468 で時間切れ (300 秒)** |
| kp から CUTPOOL_PROP を外す | 2265 (110 秒) |
| **kpnf** = kp から PROP_FIXPOINT を外す | 197 (38 秒)、349 (116 秒)、435 (43 秒) |
| min = PROOF_CMIR + PROOF_CMIR_POOL=nocap | 1377 (94 秒)、5699 (236 秒) |
| mink = min + PROOF_KEEP_INT | 1181 (90 秒)、1153 (125 秒) |

4 スレッド・60 秒 (打ち切り値なし):

| 設定 | 結果 (秒、ノード) |
|---|---|
| 既定 | 時間切れ 3 回 (3888・4520・5063 ノード) |
| kp | **最適 3 回: 58.6 (3753)、36.1 (895)、50.6 (2241)** |
| min | 時間切れ 3 回 |
| mink | **最適 3 回: 58.6 (3559)、34.7 (1209)、56.3 (3279)** |
| kpnf | **最適 3 回: 59.2 (3459)、40.2 (1777)、44.2 (1647)** |
| (HiGHS threads=4、前の調査) | 32.4・35.5 秒 (1720 ノード) |

- 4 スレッドでは木に入るのが実時間 15〜25 秒 (前の調査) で、60 秒の上限には根の時間がまだ大きい。
- `PROOF_KEEP_INT` は 1 スレッドの切り分けでは外しても解けた (2439) が、4 スレッド 60 秒では外すと 0/3 (min)。

## 6. 副作用 (40 問、4 スレッド・60 秒、既定と kpnf を 1 回ずつ)

(計測中。`logs/side/`、`side_table.txt`)

## 7. 試作・診断の一覧 (どれも既定では無効)

- `ENOMOTO_MIP_BRANCH_REPLAY=ファイル` (src/mip/replay.rs、solver.rs の木のループ、queue.rs `prio_only`): HiGHS の分枝をなぞる。診断用。
- `ENOMOTO_MIP_REPLAY_LOG`・`ENOMOTO_MIP_REPLAY_DETAIL=N` (`ER`・`ERD` 行)、`ENOMOTO_MIP_DEBUG_RESTART_CUTS` (再スタートで残すカットの
  整数列だけのものの数。この問題は 0 本)、`ENOMOTO_MIP_ORACLE_CUTS=ファイル` (他のソルバーの `HCC` 行のカットをプールへ、上限を見ない)、
  `ENOMOTO_MIP_DEBUG_CPP`、`ENOMOTO_MIP_DEBUG_PROOF=2` (証明と CMIR のカットの係数)。
- `ENOMOTO_MIP_PROOF_KEEP_INT` (solver.rs `add_proof_from`): 試作。上の §4.1。
- `ENOMOTO_MIP_PROOF_CMIR_POOL=nocap` (`add_proof_cmir`、`Solver::pool_nocap`): 試作。上の §4.2。
- `ENOMOTO_MIP_PROP_FIXPOINT` (`node_prop_fixpoint`): 試作。担当の問題では外した方が良い。
- 前回の調査 (02372a38) から移した試作: `ENOMOTO_MIP_LP_INT_BOUNDS`、`ENOMOTO_MIP_PROOF_SIDE_REDUCE`、`ENOMOTO_MIP_PROOF_CMIR`、
  `ENOMOTO_MIP_PROOF_CMIR_POOL`、`ENOMOTO_MIP_PROOF_FIXPOINT`、`ENOMOTO_MIP_HIGHS_CONFLICT_SCORE`、`ENOMOTO_MIP_CHILD_HIGHS`、`ENOMOTO_MIP_TREE_LOG`。
- カットプールの伝播の境界の変更に理由の値 −9 を付けた (`set_external_tag(7)`。衝突解析は `< 0` しか見ないので動作は同じ)。
- HiGHS の計装 (`highs_instrument_replay.patch`): `HIGHS_PATH_LOG` (`HP`・`HB`、理由の内訳 `#B..`)、`HIGHS_PATH_DETAIL=N` (`HPD`)、
  `HIGHS_CUTREASON_DUMP` (`HCUT`)、`HIGHS_ADDCUT_LOG` (`HADDC`: 出どころ 1 generateCut、2 generateConflict、3 path mixing、4 clique、5 implications)、
  `HIGHS_CONFCUT_DUMP` (`HCC`)。`Reason::unspecified` を被約費用と対称性で index 1・2 に分けた (動作は同じ)。

## 8. うまくいかなかったこと・確かでないこと

- ENOMOTO の 1 スレッドの揺れ (根のカット) は止められなかった。なぞりの木の大きさの比較は 3〜5 回の幅で見ている。
- 前回の試作 full (LP_INT_BOUNDS + SIDE_REDUCE=dense + PROOF_CMIR + PROOF_CMIR_POOL) は、なぞりの上でも効かない (3941〜10569)。理由は §4.1・4.2。
- 「HiGHS のカットを渡す」実験の 1 回目 (16099 ノード) は、カットがプールの上限で入っていなかった (計装の誤り。直した後が 21 ノード)。
- 推測: `PROOF_CMIR_POOL=nocap` は、根の密なカットでプールが一杯の問題すべてで、短い衝突カットが入るようになる。プールの非零数が
  上限なしに伸びうる (衝突カットは短いので伸びは小さいと見るが確かめていない)。
- 推測: 既定にするなら、短いカット (たとえば 64 列以下) だけ上限の外に置く、または上限を密なカットだけに掛ける形が良い。
