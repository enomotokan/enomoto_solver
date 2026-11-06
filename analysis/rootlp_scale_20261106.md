# 試作 `ENOMOTO_MIP_ROOTLP_SCALE=1` を既定にするかの確かめ (2026-11-06、途中)

bench に使う環境変数: base = なし (今の既定。安全網 `ENOMOTO_MIP_ROOTLP_SCALE_RESCUE` は既定で入っている)、
fix = `ENOMOTO_MIP_ROOTLP_SCALE=1` (条件 `ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_M` 10000・`_MIN_RANGE` 1e4 は既定値)。
どちらも `ENOMOTO_MIP_THREADS=4` (`--threads 4`)・300 秒、調べのため `ENOMOTO_MIP_LOG=1`・`ENOMOTO_SIMPLEX_SCALE_LOG=1` (ログだけで動きは変えない)。
数えの回 (1 節) は 1 スレッド・40 秒で、`ENOMOTO_MIP_ROOTLP_SCALE=1 ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_M=1000000000`
(尺度の調整は働かせず既定と同じ動きのまま、根の LP・最初の LP の行数と係数の幅だけを `ROOTLP_SCALE:` の行に出す)。

生データは `analysis/rootlp_scale_20261106/`: `candidates.json` (候補)・`meta_orig.jsonl` (元の問題の行数・係数の幅)・`scan.jsonl`
(数えの結果)・`scanlogs.tar.gz`・`b300.jsonl` (300 秒の比べ)・`logs/`。走らせ方は `fetch.py`・`scan.py`・`scan_all.sh`・`run.py`・`queue.sh`、
表は `summarize_scan.py`・`cmp.py`。調べ用に `TwoStageLp::solve_once` に `ROOTLP_SCALE:` の行 (`ENOMOTO_SIMPLEX_SCALE_LOG` のときだけ) を足した。

**この記録は途中**。300 秒の比べ (24 問) の 1 回目の 19 問までと、数えの結果で書いている。残り (uccase12・uccase9・momentum3・
neos-799711・npmv07、2 回目、閾値を変えた形) は続けて足す。

## 結論 (今の時点、短く)

- **条件に当たる問題の数 (事実)**: 前処理の後の根の LP (または前処理の LP による固定の最初の LP) が行 1 万以上・係数の幅 1e4 以上なのは、
  数えた 161 問のうち **37 問**。MIPLIB 2017 benchmark と Mittelmann の 186 問のうち元の行が 1 万以上の 103 問では 21 問
  (benchmark 240 問のうち 21 問、Mittelmann 186 問のうち 14 問)、collection の残り (行 1 万以上、非零 150 万以下の 317 問) から
  無作為に選んだ 60 問 (取得できた 58 問) では 16 問。9 問は 40 秒で根の LP まで進まず数えられていない。
- **効き目 (事実)**: 根の LP が尺度なしで重い問題ではっきり効く。momentum1 (下界・解なし → ギャップ 81.8%)、
  neos-2987310-joes (最適 31.2 → 10.1 秒)、satellites2-40 (根の LP 68 → 0.16 秒、結果は同じ)、satellites2-60-fs (根の LP 38 → 6.4 秒)。
- **害 (事実と推測)**: 根の LP がもともと軽い (1 秒前後) 問題では根の LP の時間は変わらないか少し増え (s100 0.38 → 0.81 秒)、
  300 秒の結果は良くも悪くも揺れる (map10・map16715-04・neos-3656078-kumeu・satellites2-60-fs は fix の解が悪く、
  irish-electricity・neos-4763324-toguru・neos-5195221-niemur・rd-rplusc-21 は fix の解が良い)。推測: 写しで解くと着く頂点が
  変わり、その後の道筋が変わる揺れで、尺度の調整の費用 (写しを作る・元の尺度で 1〜6 反復) は小さい。2 回目で揺れか害かを確かめる。
- **既定化の判断 (今の時点)**: 保留。効き目は根の LP が重い問題に限られ、係数の幅や行数の閾値では効く問題 (momentum1 幅 7.9e9・
  joes 3.7e6・satellites 6.6e5) と揺れだけの問題 (map 9.9e6・kumeu 1.5e7) を分けられない。既定にするなら
  「尺度なしの根の LP が重いときだけ写しに切り替える」形 (下の 5 節の案) の方が害が小さいと見ている。

## 1. 条件に当たる問題を数える

候補: MIPLIB 2017 collection の表 (`tag_collection.html`) から元の行が 1 万以上の問題を選んだ (前処理は行をほぼ増やさないので、
元の行が 1 万未満なら根の LP も 1 万未満と見なした。これは推測で、クリーク行の拡張は行を減らす向き)。benchmark・Mittelmann の
103 問 (`bench_mitt`) と、その他の 317 問から無作為に 60 問 (`coll_sample`、種 20261106)。

| 組 | 数えた | 条件に当たる (行 ≥ 1e4・幅 ≥ 1e4) | 40 秒で根の LP まで進まない |
|---|---:|---:|---:|
| benchmark・Mittelmann (元の行 ≥ 1 万) | 103 | 21 | 1 (neos-3004026-krka) |
| collection の無作為 60 問 (取得に失敗した 2 問を除く) | 58 | 16 | 8 |
| 計 | 161 | 37 | 9 |

閾値を変えたとき (同じ 161 問、`python summarize_scan.py 行 幅`):

| 行の閾値 | 幅の閾値 | 当たる問題 |
|---:|---:|---:|
| 1e4 | 1e3 | 54 |
| 1e4 | 1e4 (今) | 37 |
| 1e4 | 1e5 | 28 |
| 1e4 | 1e6 | 19 |
| 2e4 | 1e4 | 28 |
| 5e4 | 1e4 | 15 |

当たった 37 問 (`scan_summary.txt`): app1-2, irish-electricity, map10, map16715-04, momentum1, neos-1122047, neos-2987310-joes,
neos-2746589-doon, neos-3656078-kumeu, neos-4722843-widden, neos-4763324-toguru, neos-5188808-nattai, neos-5195221-niemur,
rd-rplusc-21, s100, satellites2-40, satellites2-60-fs, snp-02-004-104, triptim1, uccase12, uccase9 (以上 benchmark・Mittelmann)、
dws008-03, dws012-03, eva1aprime6x6opt, momentum3, neos-4165869-wannon, neos-4232544-orira, neos-4290317-perth, neos-4760493-puerua,
neos4, neos-799711, neos-914441, npmv07, ns930473, polygonpack4-10, satellites3-25, supportcase1 (collection)。

- 事実: 当たった問題の多くは、既定 (尺度なし) の根の LP が 1 秒未満で終わる (数えの回の 1 スレッドで 37 問中 16 問)。
  根の LP が 5 秒を超えるのは snp-02-004-104 (16.5 秒)・neos-4165869-wannon (6.5 秒) と、40 秒 (1 スレッド) で根の LP が終わらない 12 問
  (irish-electricity・momentum1・s100・satellites2-40・satellites2-60-fs・triptim1・uccase12・uccase9・eva1aprime6x6opt・momentum3・
  neos-4232544-orira・satellites3-25)。

## 2. 300 秒の比べ (4 スレッド、1 回目、`b300.jsonl`)

「LP」は根の LP の秒 (`MIP: root LP` の行)、rb は根の下界、obj は解、ギャップは (解 − 最後の下界) / |解|。

| 問題 | base | fix |
|---|---|---|
| momentum1 | 時間切れ、根の LP が終わらず、下界・解なし | 時間切れ、LP 0.54 秒、rb 96265.5、解 527462.9 (ギャップ 81.8%) |
| satellites2-40 | 時間切れ、LP 68.2 秒、rb -30、解 5 | 時間切れ、LP 0.16 秒、rb -30、解 5 (最初の解 102 → 52 秒) |
| app1-2 | 時間切れ、LP 0.45 秒、rb -132.0、解なし | 時間切れ、LP 0.59 秒、rb -71.4、解なし |
| irish-electricity | 時間切れ、rb 3636262、解 3895602 (6.66%) | 時間切れ、rb 3633086、解 3850001 (5.63%) |
| map10 | 時間切れ、rb -587.6、解 -364 | 時間切れ、rb -588.2、解 -308 |
| neos-1122047 | 最適 1.5 秒 | 最適 1.6 秒 |
| map16715-04 | 時間切れ、rb -296.3、解 -85 | 時間切れ、LP 0.22 秒、rb -296.3、解 -53 |
| neos-2987310-joes | **最適 31.2 秒** | **最適 10.1 秒** |
| neos-2746589-doon | 時間切れ、LP 0.45 秒、解なし | 時間切れ、LP 0.27 秒、解なし |
| neos-3656078-kumeu | 時間切れ、LP 1.82 秒、解 -12332.5 | 時間切れ、LP 1.27 秒、解 -11905.3 |
| neos-4722843-widden | 時間切れ、LP 2.73 秒、rb 10361.8、解 25009.66 | 時間切れ、LP 2.40 秒、rb 10797.1、解 25009.66 |
| neos-4763324-toguru | 時間切れ、LP 0.17 秒、解 2091.34 | 時間切れ、LP 0.25 秒、解 2031.30 |
| neos-5188808-nattai | 時間切れ、解 0.1204 | 時間切れ、解 0.1277 |
| neos-5195221-niemur | 時間切れ、解 0.00907 | 時間切れ、解 0.00830 |
| rd-rplusc-21 | 時間切れ、解 173308.5 | 時間切れ、解 173076.5 |
| s100 | 時間切れ、LP 0.38 秒、解 -0.14315 | 時間切れ、LP 0.81 秒、解 -0.14221 |
| satellites2-60-fs | 時間切れ、LP 38.4 秒、rb -29.875、解 -2 | 時間切れ、LP 6.4 秒、rb -29.875、解 1 (最初の解 72 → 16 秒) |
| snp-02-004-104 | 時間切れ、解 586803273 | 時間切れ、解 586805486 |
| triptim1 | 時間切れ、LP 49.3 秒、解なし | 時間切れ、LP 51.0 秒、解なし |

- 事実: irish-electricity・map10・map16715-04 (base) はログが途中で切れていて、根の LP の時間が取れていない。計測中に自分のコミットが
  書き途中のログを `git add` し、rebase で作業ファイルが置き換わって書き先が消えたため (計測の結果そのものには関わらない)。
  直してからはログをリポジトリの外に書き、終わってから `logs/` に gzip で置く。
- 事実 (`SIMPLEX_SCALE:` の行): 写しから元の尺度へ戻すときの反復はどの問題でも 1〜6 反復・0.1 秒前後で、戻しの費用は小さい。
- 事実: 写しの求解が `Error` で終わる行が 2 問にある (neos-2746589-doon の最初の LP、triptim1 の根の LP)。どちらも基底のない求解で、
  二段解法と内点法の競争 (`TwoStageLp::solve` の race) の負けた側が取り消されたもの (続く「unscaled cold Error 0 iters」も取り消し)。
  根の LP そのものは競争の相手が解いている (triptim1 は base と同じく 50 秒前後)。推測: 基底のない根の LP では尺度の調整は二段解法の側に
  しか効かず、内点法が勝つ問題では効き目がない。
- 事実: 写しの求解が反復の上限 (`IterationLimit`) で止まった行が 3 問 (s100・snp-02-004-104・triptim1 の最初の LP)。前処理の LP による
  固定の最初の LP の上限で、尺度なしでも同じく止まる。
- 事実 (satellites2-60-fs): fix の根の LP は最初の LP の基底から写しで 9902 反復 (6.4 秒)。base は基底なしで 47734 反復 (38.4 秒)。

## 3. 悪くなった問題の原因 (途中)

- map10・map16715-04: 最初の LP は base でも写しでもほぼ同じ時間 (数えの回の base 10.2・11.9 秒、fix 10.8 秒) で、根の LP は 0.2〜0.3 秒、
  根の下界もほぼ同じ (-587.6 / -588.2、-296.27 / -296.27)。違うのは着く頂点 (最初の解の時刻 32.5 → 41.1 秒、34.8 → 40.5 秒)。
  推測: 尺度の調整の費用ではなく、頂点の違いによる揺れ。2 回目で確かめる。

## 4. 閾値を変えた形

(続けて足す)

## 5. 既定にするなら (案、今の時点)

(2 回目と閾値の結果を見てから書く)
