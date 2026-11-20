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
