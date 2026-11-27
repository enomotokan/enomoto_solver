# 根の下界 (gap240 の型 3) の切り分けと試作 (2026-11-26、親の依頼)

analysis/gap240_20261126 (HsTC の調べ) の型 3。istanbul-no-cutoff (SCIP 129)・net12 (HiGHS 130)・neos-873061 (HiGHS 113.4) で、当方の根の下界が届かない理由を切り分ける。
過去の調べ: analysis/cutgap_20261123 (3 者の根の比べ、ENUM_ADAPT・PRR_TIGHT・SCIPAGG)、analysis/aggrsel_20261121 (集約の相手の選び方)。

## 台本

- `ours_root.py 問題 [名札] [ENV=値 ...]` (analysis/cutgap_20261123 の写し): 当方の根だけ (1 スレッド・種 1・ノード上限 1)。`OURS_MPS` で別の MPS、`OURS_TL` で上限。→ `ours.jsonl`・`logs/`
- `cfg.py`・`plan.py`・`run.py`・`summ.py`: P の比べ (analysis/rootpar_cut_20261126 の写し)。

## 1. 今の根 (ffa74c958、1 スレッド・種 1・根だけ)

| 問題 | 既定 | + ENUM_ADAPT=1s | 相手 |
|---|---|---|---|
| net12 | 79.5 (114 秒) | 131.8 (79 秒) | HiGHS 130 |
| istanbul-no-cutoff | 71.0 (72 秒) | 80.0 (45 秒) | SCIP 129 |
| neos-873061 | 104.7 (327 秒) | 104.7 | HiGHS 113.4 |

## 2. 原因の切り分け

- **net12: 前処理 (列挙) だけ。** ENUM_ADAPT (c2c の部品) で HiGHS を超える。
- **istanbul-no-cutoff: 前処理 (約 16) とカットの族 (約 36)。**
  - SCIP の前処理後の問題 (`../cutgap_20261123/presolved_scip/`) を当方が解くと、25 ラウンドの上限で 86.5 (まだ伸びている)。上限 100 ラウンドで 96.3 (52 ラウンドで停滞)。SCIP は 132。
  - 元の問題では、上限を 100 ラウンドにしても延びない (既定 68.9・ENUM_ADAPT つき 80.0 で停滞)。ラウンドの上限で止まるのは、前処理の良い問題だけ。
  - 前処理: ENUM_ADAPT の繰り返しでプロービングが周ごとに 904・1268・1250・876・… 本の境界を締めるが、根の LP は 60.94 で止まる (SCIP の前処理後 66.83)。
- **neos-873061: LP の速さ (→ ラウンドの数) と再スタート。** 前処理の差は小さい (HiGHS は 3347 列を減らしただけ)。

| | 当方 (1 スレッド) | HiGHS (4 スレッド) |
|---|---|---|
| 根の LP | 95.91 (43812 反復・111 秒) | 98.73 (3831 反復・8.9 秒) |
| カットの後 | 104.7 (8 ラウンド、根の終わり 225 秒) | 113.0 (33 秒) |
| 良い解 | 125.9 (123 秒)・121.5 (249 秒) | 122.4 (17 秒)・113.8 (58 秒) |
| 再スタート | なし (固定 0 本) | 58 秒で 88% の整数列が動かず再スタート、6 回で 4558 列 |

  - 大元は根の LP の反復 (12 倍)。LP の速さの調べ (HJZ2) の領域。

## 3. 試作 `ENOMOTO_MIP_PATHH_SLACK` (既定オフ、src/mip/separation.rs の `pathh_slack`)

- 経路集約 (path_highs、HiGHS の移植) は、両側にすき間のある行を出発にも相手にも使わない (`pathh_row_types` で型 0)。
  SCIP の sepa_aggregation は、すき間のある行も近い側を選んで使う (aggrsel_20261121 の 3 節の表)。
- 試作: すき間のある行も、近い側の不等式として経路に使う。すき間は cMIR の中で連続変数として扱われる。新しい閾値はない。
- 過去の AGGRSEL (相手の行の選び方)・SCIPAGG (止め方) とは別の所 (使える行の範囲)。

| 問題 (1 スレッド・種 1・根だけ) | 既定 | + PATHH_SLACK | 相手 |
|---|---|---|---|
| istanbul-no-cutoff | 71.0 (72 秒) | **91.9** (63 秒) | SCIP 129 |
| istanbul-no-cutoff + ENUM_ADAPT | 80.0 (45 秒) | **92.4** (63 秒) | |
| istanbul-no-cutoff (SCIP の前処理後、上限 100 ラウンド) | 96.3 | **108.3** | SCIP 132 |
| net12 | 79.5 (114 秒) | 85.0 (52 秒) | HiGHS 130 |
| neos-873061 | 104.74 (327 秒) | 104.96 (302 秒) | HiGHS 113.4 |

## 4. P の比べ (`shards.json`、4 分担、既定と `ps` = PATHH_SLACK、1・4 スレッド、種 1〜3、60 秒)

- ビルド ab631db23 (egj・UNIFY_FIX の z・p・LPPROBE_FIX=d が既定、check_default 一致)。
- 回し方 (K = 0〜3、リポジトリの根から):

```
git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD
pip install -e .
python analysis/protoscreen_20261124/check_default.py
python analysis/rootbound_20261126/run.py K analysis/rootbound_20261126/shards.json analysis/rootbound_20261126/res_sK.jsonl
```

- 集計: `python analysis/rootbound_20261126/summ.py 'analysis/rootbound_20261126/res_s*.jsonl'`

### 結果 (`res_s{0,1,2,3}.jsonl`・`summ.txt`、4 分担、src は ab631db23 = f3b957e68)

- 696 回、crash 0、食い違い 0。分担 1〜3 は GcYE・EwwjY・FinV (親の依頼)。

| スレッド | 比 | 95% 区間 | 解けた数 既定 → ps | 全種 1.25 倍超の遅れ | 目安 |
|---|---:|---|---|---|---|
| 1 | 1.003 | [0.981, 1.025] | 120 → 120 | neos-3083819-nubu [1.49, 1.36, 2.71] | 外れ (遅れ) |
| 4 | 0.997 | [0.980, 1.014] | 137 → 138 | なし | 候補 |

- 効いた問題 (問題単位の比): pk1 0.80、h80x6320d 0.82、swath1 0.82、rout 0.85、4 スレッドの noswot 0.75・10teams 0.80。
- 1 スレッドの neos-3083819-nubu (4 秒ほどの小さい問題): 既定 4.0・4.25・3.2 秒、ps 5.4・11.5・4.8 秒 (種 2 でノード 132 → 981)。
  - 根だけ (`ours_root.py`): 下界 6307065 → 6306658、根の時間 3.8 → 4.5 秒。経路集約はどちらの形でもカットを採らない (chosen 0)。
  - すき間の行で試す経路が増えて (work 38 → 602)、カットの並びと時間が少し変わり、木の道筋が変わったと見る。4 スレッドは同じ時間 (2.2 秒前後)。
- physiciansched6-2・mas76 (分担 0 の途中で目立った) も、根の下界と根の時間はほぼ同じ (`ours.jsonl`)。木の道筋の違い。
- 見立て: P では中立 (1 スレッド 1.003・4 スレッド 0.997)。対象の istanbul-no-cutoff (根 71 → 92)・net12 (79.5 → 85) は根が上がる。
  効き目のシグモイドでの適応は、仕組みの害が見つからなかったので入れていない。
