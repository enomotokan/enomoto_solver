# cand1111 の確かめ (2026-11-11)

束 cand1111 = 既定 + ROOTSTOP=b, RCSTRONG=irk, LMIP_SWAP=1, TREEFIX=pcu, SYMPRE=probe,
CUTCONF=1, PLSB=gxu, GAP1=u, GAP4=rocn (いずれも ENOMOTO_MIP_ 接頭)。ビルドは 08d087d5 に固定。
既定と束は同じ分担 (同じ機械) で対にして回した。

## MIPLIB 58 問 × 種 1〜4、60 秒 (res_P_s*.jsonl、集計 summP.py)

- 464 行、crash 0、最適値の食い違い 0。
- 解けた数: 既定 173 / cand1111 172 (noswot・mc11・physiciansched6-2 で 1 回ずつ失い、h80x6320d・timtab1 で 1 回ずつ得る)。
- 対の比 (shift 1 秒の SGM、どちらかが解けた 45 問): **−2.5% [−6.8%, +1.6%]**。
- 全種で 1.25 倍以上遅い問題はなし (やや遅め: markshare_4_0、pk1)。速い側: p200x1188c、khb05250、exp-1-500-5-5、neos5。
- 最大 RSS: 既定 1863 MB / cand1111 1390 MB。
- 判定: 取り込み基準 1〜3 を満たす。Mittelmann 600 秒と誤答検査の結果待ち。

## Mittelmann 186 問 × 種 1・2、600 秒 (res_M_s*.jsonl、集計 summM.py → summM.txt)

- 744 行、crash 0、最適値の食い違い・下界の逆転 0。M41〜M49 は起動の割り当ての失敗で立て直した (同じ固定のコミット)。
- 解けた (372 回): 既定 141 / cand1111 139。SGM (shift 10): 223.48 → 232.68。
- 対の比 (どちらかが解けた 75 問): **+10.3% [+1.0%, +22.3%]** → **取り込みの基準を満たさない** (害)。
- 2 種とも遅い: hypothyroid-k1 (2.6・2.9 倍、2 種とも解けなくなる)、neos859080 (2.5・22 倍)、mas74 (2.0・1.7)、beasleyC3 (1.4・1.4)、
  gen-ip054 (1.3・1.4)、n2seq36q (1.4・5.4)。2 種とも速い: pg5_34 (0.33・0.39、2 種とも新しく解ける)、neos-2978193-inde (0.47・0.53)。
- HiGHS だけが解く (延べ): 既定 40 / cand1111 42。最大 RSS: 既定 7811 MB / cand1111 5429 MB (GAP4 の効き)。
- 判定: 58 問では基準を満たしたが、Mittelmann で害があるので cand1111 は既定にしない。害の出た問題と速くなった問題で、
  既定に 1 項目だけ足した形 (only_*) を比べて原因の項目を探す (plan_B.py → shards_B.json、20 分担 × 10 回、600 秒)。
