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
