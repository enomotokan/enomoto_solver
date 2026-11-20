# hypothyroid-k1 の大 M の行と SCIP の決め手 (2026-11-20、SCIP-GATE の担当)

## 1. SCIP の切り分け (SCIP 10.0.2、1 スレッド、種 0)

- and 制約・gate extraction は決め手ではない。
  - 既定は 8.2 秒。gate extraction を切ると 6.5 秒、knapsack の分解を切ると 5.0 秒で、どちらも速くなる。
  - constraints/and/linearize・aggrlinearization・enforcecuts は、どれも 8.0〜8.2 秒で最初の LP も同じ。
- 前処理は cons_linear だけで足りる。knapsack・logicor・setppc への格上げと分解を全部切ると 1.9 秒。
  ただし前処理後の問題そのものの LP は −2903 で、当方の LP と同じ (HiGHS で確認)。
- 本当の差は、根で最初の LP の前に 1318 列を固定していること。
  - vbounds の発見法 (変数上下限とクリーク表の辺の含意グラフを位相順に並べ、その順に固定と伝播をする) が −2633 の解を出す。
  - pseudoobj (クリークの含意を使う目的の伝播) が 67 列、続く行の伝播が 1251 列を固定する。
  - 最初の LP は 2122 反復・0.26 秒で −2851.08 (最適 −2851)。
- linear の前処理だけの設定で切ると、vbounds なしで 8.3 秒、pseudoobj なしで 120 秒で終わらない。
- −2633 の解を外から与えて vbounds を切ると 1.5 秒 (前処理で行が 5189 → 2553)。
- 当方 (今の HEAD、1 スレッド) は 10.1 秒 (記録の 28.5 秒より速い)。
  - 前処理の LP probing が根の LP を丸ごと解く (3.3 秒、2607 反復)。
  - 良い解 −2851 は根の LP の後の乱択丸めで出て、OBJPROBE p が 68 列を固定して閉じる。

## 2. 試作 `ENOMOTO_MIP_VBCLQ` (既定では使わない、案 B')

- 置き場所: 根の LP の直前 (solver.rs、`Solver::vbclq_pseudoobj`)。暫定解か打ち切り値 (`params.cutoff`) があるときだけ働く。
- 中身は SCIP の pseudoobj の propuseimplics の形。
  - 文字を真にしたときの目的の損 = 自分の損 + 衝突グラフで偽になる列の損。
  - 損が余裕 (打ち切り値 − 目的の最小値) を超える文字を偽にして、大域的に固定する。
  - 伝播はしないので、手間は辺の数に比例する。固定が出なくなるまで回し (間に行の伝播を挟む)、10 回まで。
- 目的の配り直し (案 B' の (a)): 前処理は変えず、この数え方の中だけで行う。
  目的を持つ列が 1 本の等式にしか出なければ、目的をその等式で他の列に配る。
  - hypothyroid-k1 では当方の前処理が既に目的の列を 0-1 列 2503 本に配り直していて、0 列。
- 仮の発見法 `h` は捨てた (発見法は SCIP-VB の担当)。
- hypothyroid-k1 (1 スレッド・種 1、`ENOMOTO_MIP_CUTOFF=-2633` で SCIP の vbounds の解の値を上限に与えた確かめ):

| 設定 | 根の前の固定 | 根の LP | 時間 |
|---|---|---|---|
| 上限だけ | 0 | −2902.85 | 10.1 秒 |
| `VBCLQ=1` | 67 列 (SCIP と同じ数) | −2851.08 (SCIP と同じ) | 8.7 秒 |
| `VBCLQ=1` + `ENOMOTO_MIP_NO_LP_PROBE=1` | 67 列 | −2851.08 (1194 反復・0.99 秒) | **3.1 秒** |

- 課題: 前処理の LP probing が、固定の前に根の LP を丸ごと解く (3.3 秒)。
  発見法の解と固定を LP probing の前に出すか、良い解があれば LP probing を省く形が要る。
- 開始解 (`ENOMOTO_MIP_START_FILE`、SCIP の −2633 の解) は、当方の前処理後の問題で実行不能として捨てられる (双対の簡約で外れる)。
  そのため確かめは打ち切り値で行った。

## 3. 大 M の行の数え上げ (`bigmscan.py`、結果 `bigmscan.jsonl_K`)

- 数えるのは、0-1 列 y が 1 なら同じ行の他の 0-1 列が全て 0、y = 0 なら行が冗長になる行。
- 元の問題で数える。負の係数の列は補って (x → 1 − x) から判定する。

- 結果 (P 58 + M 145 = 203 問、誤り 0): 大 M の行がある問題は 23。非零の 1 割以上を占めるのは 15 問。
  - P: hypothyroid-k1 (49.7%)・neos8 (28.3%)・neos-827175 (11.6%)・physiciansched6-2 (15.0%)。
  - M: neos-787933 (79.2%)・neos-4387871-tavua・neos-1445765・bnatt400・bnatt500・peg-solitaire-a3 (45〜49%)・
    pg5_34 (33.8%)・cryptanalysiskb128n5obj14・obj16 (30.4%)・decomp2 (16.6%)・brazil3 (15.5%)。
  - 少ないもの (非零の 1 割未満): misc07・roll3000・neos-3656078-kumeu・neos-4722843-widden・uccase9・uccase12・rocII-5-11・
    academictimetablesmall。

## 4. VB01 との組み合わせ (hypothyroid-k1、1 スレッド・種 1)

- VB01 は SCIP-VB の試作 `ENOMOTO_MIP_VB01=b` (根の LP の前の vbounds)。−2692 を見つける (SCIP の既定と同じ値)。

| 設定 | 時間 |
|---|---|
| 既定 | 10.2 秒 |
| `VB01=b` | 10.5 秒 |
| `VB01=b` + `VBCLQ=1` (67 列を固定、根の LP −2851.08) | 8.85 秒 |
| `VB01=b` + `VBCLQ=1` + `ENOMOTO_MIP_NO_LP_PROBE=1` | **4.2 秒** |

- 確かめ: fuzz.sh (`VBCLQ=1`) の 6 本で誤答 0。
  - tiny 0/400・highs 0/300・objint 0/1000・slack 0/1000・tiny2 0/400・objint2 0/1000。
  - fuzz の中で固定が何回働いたかは数えていない。
- 働く問題の洗い出し: `workscan.py` (`VB01=b` + `VBCLQ=1`、1 スレッド・1 問 120 秒まで) → `workscan.jsonl_K`。

## 5. `l` (前処理の LP による固定をしない) と fuzz の働いた回数

- `ENOMOTO_MIP_VBCLQ=l`: pseudoobj に加えて、前処理の LP probing をしない (親の判断で案 (iii) を入り切りの形にした)。
- hypothyroid-k1 (1 スレッド): `VB01=b` + `VBCLQ=l` で 3.6 秒・3.3 秒 (種 1・2)。既定は約 10 秒。
- fuzz.sh (計測の vbcl と同じ `VB01=b` + `VBCLQ=l`)。働いた回は診断 `ENOMOTO_MIP_VBCLQ_COUNT_FILE` で数えた。

| fuzz | 誤答 | pseudoobj の呼び出し | 固定した回 (列) | 打ち切りの証明 |
|---|---|---|---|---|
| tiny (400) | 0 | 157 | 148 (322) | 133 |
| highs (300) | 0 | 20 | 1 (2) | 1 |
| objint (1000) | 0 | 1305 | 40 (61) | 25 |
| slack (1000) | 0 | 365 | 9 (10) | 179 |
| tiny2 (400) | 0 | 176 | 157 (356) | 138 |
| objint2 (1000) | 0 | 1304 | 34 (47) | 34 |

- 固定が働いた回は合わせて 389 回 (列 798 本)。打ち切りの証明は 510 回。誤答 0。
  - 証明は、余裕が負 (目的の最小値が打ち切り値を超える) か、固定の後の伝播が矛盾した回。
- 計測の台本: cfg.py (def・vb・vbc・vbcl)・plan.py・run.py・summ.py (`python summ.py P vbcl vb` のように基準も選べる)。
  M の対象は workscan.py の結果で決める。

## 6. `e`: LP probing の前に固定する (利用者の案)

- pseudoobj の本体を、Solver の外で回せる関数 `vbclq::pseudoobj` (src/mip/vbclq.rs) に切り出した。
  Solver の `vbclq_pseudoobj` はその包み。
- `ENOMOTO_MIP_VBCLQ=e` の流れ (presolve_mip_depth の中、前処理の LP probing の直前):
  1. 暫定解の値ちょうどを上限にして固定する (暫定解は残る)。
  2. 固定したら前処理をかけ直す (quick_probe の固定と同じ形)。
  3. かけ直した問題で LP probing を回す。
- 暫定解の出どころ: VB01 の Solver の外の版を SCIP-VB に依頼中。それまでは診断 `ENOMOTO_MIP_VBCLQ_PRE_CUTOFF=<値>` で上限だけ与える。
- hypothyroid-k1 (1 スレッド、`VB01=b` + `VBCLQ=e` + 上限 −2692 = VB01 が出す値):
  - 0.65 秒・0.66 秒 (種 1・2)。vbcl は 3.86 秒・3.27 秒。
  - 固定は 67 列 (伝播を含めて 71 列が変わる、0.03 秒)。
  - かけ直しで行 5189 → 1733 → 899、非零 431326 → 3930。
  - LP probing の根の LP は 19 反復・0.00 秒 (前は 2607 反復・3.3 秒)。LP probing の固定 0 は前と同じ。
  - さらにかけ直して行 227・自由な列 121。根の LP は 34 反復で −2851 (最適)。
- P の計測 (親、分担 2) では、vbcl (LP probing を省く) が ns1116954 種 1 で 2.6 → 35.4 秒、neos8 種 3 で 5.2 → 16.4 秒と大きく遅れた。
  l は害が大きい。本命は e で、確かめの組に ns1116954・neos8 を入れる。

## 7. P の計測 (58 問 × 種 1〜3 × def・vb・vbc・vbcl、60 秒・4 スレッド、4 分担がそろった 7248fa14e、別のセッション)

集計は `summ_P.txt` (`summ.py P <比べる設定> <基準>`)。crash 0、食い違い 0。

| 比べ | SGM [95% 区間] | 解けた数 | 全種で 1.25 倍超遅い問題 |
|---|---|---|---|
| vb / def | +4.4% [+0.2, +9.9] | 139 → 140 | nw04 2.51 |
| vbc / def | +4.7% [+0.6, +10.0] | 139 → 139 | nw04 2.44 |
| vbcl / def | +8.1% [−2.3, +21.0] | 139 → 140 | ns1116954 6.21・neos8 2.82・nw04 2.39・neos-3083819-nubu 1.62・drayage-100-23 1.45 |
| vbc / vb | +0.3% [−1.7, +2.3] | 140 → 139 | なし (hypothyroid-k1 0.83) |
| vbcl / vbc | +3.3% [−5.8, +14.7] | 139 → 140 | ns1116954 5.62・neos8 2.73・neos-3083819-nubu 1.60・drayage-100-23 1.47 |

- pseudoobj (vbc / vb) は中立 (+0.3%)。hypothyroid-k1 は 0.83 倍。
- LP probing を省く l は害が大きい (ns1116954・neos8 など)。全種で速いのは hypothyroid-k1 0.35 と physiciansched6-2 0.65 だけ。
  → 本命は LP probing の前に固定する e (6 節)。
- nw04 は vb 系の 3 形とも 2.4〜2.5 倍遅い (VB01=b 由来)。
  親の見立ては「早い暫定解 → 再スタートの費用が大きい」で、P2-HEUR が直し中。
- neos-1456979 の種 3 で、暫定解が vb 系で大きく悪い (200 → 378〜424)。
  - 手元 (4 スレッド・種 3・60 秒) では、VB01 自身は 0.10 秒で何も見つけない。
  - 既定では根の Local-MIP が 1.5〜2.2 秒で 382 → 352 を出すが、vb では Local-MIP の改善が 0 回 (LMBOOST の行) になる。
    最初の解は 21.5 秒の robust FJ の 433、終わりは 424。
  - 根の前の VB01 で、並列の Local-MIP の流れ (始める時刻・状態) が変わったためとみられる (SCIP-VB に確かめを依頼)。

## 8. e に VB01 の解をつなぐ (SCIP-VB の `vbounds01_on_problem`、c66aa84b4)

- `e` は、前処理の LP probing の直前に `heur_scip::vbounds01_on_problem` で解を探す。
  - サブ MIP 1 本の時間の上限は、LP probing の時間の上限と同じ安全網。サブ MIP は 500 ノードまで。
- 見つけた解の値を上限にして pseudoobj で固定し、前処理をかけ直す。
- 解は前処理の後の開始解 (START_SOL) として B&B に渡す (前処理後の問題で実行可能なときだけ)。
- 計測の vbce は `ENOMOTO_MIP_VBCLQ=e` だけにする (根の前の `VB01=b` は付けない。nw04 の遅れが `b` 由来のため)。
- 確かめ (4 スレッド・60 秒、秒):

| 問題 | def | vb (VB01=b) | vb + e | e だけ (種 1 / 2) |
|---|---|---|---|---|
| hypothyroid-k1 | 9.81 | 10.14 | 1.02 | **1.12 / 1.18** |
| ns1116954 | 3.38 | 3.32 | 3.39 | 3.56 / 3.43 |
| neos8 | 5.51 | 5.86 | 5.65 | 5.58 / 5.97 |
| nw04 | 14.98 | 38.55 | 39.37 | 16.65 / 15.42 |

- 各問題での e の働き:
  - hypothyroid-k1: VB01 の解 −2692 で 67 列を固定する。
  - neos8: VB01 の解 −3223 があるが固定 0 (余裕 40927)。
  - nw04: 前処理の中の VB01 は解なし。
  - ns1116954: e の行が出ない (前処理の早い FJ が解を見つけて前処理を切り上げる道)。
- fuzz の数えるファイルに出どころ (r = 根の LP の前、e = 前処理の LP による固定の前) を足した。

## 9. fuzz で見つけた誤り (e) と直し、2 回目の計測の分担表

- 直す前の fuzz_tiny (`VBCLQ=e`) で誤答 2 (種 64・362)。どちらも「HiGHS は最適、当方は実行不能」。
  - 原因: VB01 の解は許容誤差 1e-6 の内で実行可能なだけで、厳密な最適より良い値のことがある。
    幅 1e-9 の列や右辺 1 − 1e-9 の行の問題で起きる。
  - その値を上限に固定すると、厳密に実行可能な点が全て消え、前処理のかけ直しが「実行不能」と言っていた。
- 直し: e の固定の後のかけ直しが実行不能と言ったら、それは信じず、固定の前の問題に戻して続ける。
  VB01 の解は開始解としてそのまま渡す。
- 直した版の確かめ:
  - fuzz_tiny 0/400・tiny2 0/400・highs 0/300・objint 0/1000・slack 0/1000、objint2 は回し中。
    前処理の中の e は tiny で 260 回呼ばれ 211 回固定、tiny2 で 270 回呼ばれ 211 回固定。
  - cargo test 414 通過。hypothyroid-k1 は 4 スレッド 1.13 秒のまま。
- 2 回目の計測 ({def, vbce}、vbce = `ENOMOTO_MIP_VBCLQ=e` だけ):
  - `python plan.py 6 3` → shards_P2.json (58 問 × 種 1〜3、60 秒、3 分担 × 0.99 時間)・shards_M2.json (6 分担 × 3.7 時間)。
  - M2 は e が働きうる 43 問の和。内訳 (重なりあり):
    - VB01 が根の前に解を出す 27 問 (workscan)
    - pseudoobj が固定した 2 問 (mzzv11・neos-957323)
    - 大 M の行がある 17 問 (bigmscan)
  - 種 1・2、600 秒。
  - 回し方: `python analysis/scipgate_20261120/run.py P2 K` (K = 0〜2)・`run.py M2 K` (K = 0〜5)。
  - 集計: `python analysis/scipgate_20261120/summ.py P2 vbce def` (M2 も同じ)。
