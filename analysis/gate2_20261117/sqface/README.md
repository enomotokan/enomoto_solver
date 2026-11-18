# gate2 候補 9: CUTSEL_DCD と SEP_FAST + FACE_ADAPT の入り切り (2026-11-18)

BRIEF.md の候補 9。進め方の (1) 特徴の洗い出しと (2) 見込みまで。**見込みが薄いので (2) で区切る**。

## 調べたこと

1. **CUTSEL_DCD の試作コードは今のリポジトリにない**。`git log --all -S CUTSEL_DCD -- src` は空で、試作のブランチ
   (inv/dircutoff-20261022 の 53f58b6・0815ea4・15f5b06) は手元にも origin にも残っていない。残っているのは記録
   (analysis/dircutoff_20261022.md、docs/improvement_history.md の 2026-10-22・23) と台本・ログだけ。
   入り切りを作るには `select_cuts_hybrid_dcd` (SCIP の式 + DCD_T) を今の選択 (固定費用型の緩めた選択・CUTSEL_HIGHS など) の上に
   作り直す必要がある。
2. **今の既定では、面の頂点の分離 (FACE_SEP) はほとんどの問題で動いていない**ので、FACE_ADAPT は実質何もしない。
   `ENOMOTO_MIP_CUT_AGG` の既定 `f` は、列 ≦ 100 × 行の問題で面の分離を止める (separation.rs の `agg_face_off`)。
   既定・種 1・30 秒で `ENOMOTO_MIP_DEBUG_SEPPROF=1` を見ると (`feat1.sh`、ログは `logs/`)、13 問のうち面の LP を解いたのは
   nw04 (列/行 2430) だけ (18 回、1 ラウンド 1133 ms のうち 99 ms)。ほかの 12 問 (10teams・roll3000・mc11・air03 (列/行 87)・
   h80x6320d・rout・noswot・qiu・neos-911970・sp150x300d・mas76・ns1116954) は 0 回。
   - FACE_ADAPT が効いた記録 (10teams 27→5 秒・drayage・h80x6320d・rout、2026-10-23) は、CUT_AGG=f が既定になる前
     (analysis/cand9bisect_20261024.md) のもの。
   - revisit_20261112 の face (+3.3% [−1.2, +9.5]、84→82) の問題ごとの差 (roll3000 0.60・10teams 0.76・ns1116954 3.66・
     noswot 1.54 など。2 種) は、働いていない設定の差なので揺れとみる。
3. **SEP_FAST は CMIR の効き目の評価を HiGHS の形にする速さだけの変更で、カットは同じ** (cuts.rs のテスト
   `cmir_terms_fast` が従来と同じカットを作ることを確かめる)。問題ごとに入り切りする理由がない。
   単独で測るなら、入り切りではなく既定化の候補として (浮動小数の丸めの差だけを見る) 別に扱うのがよい。

## 見込み

- FACE_ADAPT: 働く問題が列/行の極端な問題 (nw04 など) に限られ、効く・遅れる群を分ける対象がない。試作しない。
- SEP_FAST: 結果を変えない高速化なので、シグモイドでの入り切りの対象ではない。
- CUTSEL_DCD: 効く群 (neos-911970・10teams・beasleyC3、退化した LP で暫定解が根のループ中にある) と遅れる群
  (mik-250・mc11・noswot・timtab1・neos-960392) を分ける候補の特徴はある。根のループの途中で暫定解があるか、LP の双対退化の割合
  (被約費用 0 の非基底列の割合) が候補。ただしコードの作り直しが要り、記録上の差の多く (mc11・neos-960392) は揺れの大きい問題で、
  「全種で速い・全種で遅い」が分かれていない (DTa・DTb で mc11 は解けたり落ちたり)。見込みは薄い。
