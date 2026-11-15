# cand1112 の確かめ (2026-11-11)

束 cand1112 = cand1111 から CUTCONF・TREEFIX を外した 7 項目 (cfg.py)。ビルドは cand1111 と同じ固定のコミット 08d087d5。
分担は cand1111 と同じ作り (plan.py: Mittelmann 186 問 × 種 1・2 × {def, cand1112} 600 秒を 50 分担、58 問 × 種 1〜4 × {def, cand1112} 60 秒を 6 分担。
同じ (問題, 種) の def と cand1112 は同じ分担)。run.py は cand1111 のものと同じ (python run.py {M|P} K)。

## P (MIPLIB 58 問 × 種 1〜4、60 秒) の結果 (summP.py → summP.txt)

- 行数 464、crash 0、食い違い (誤答・矛盾) 0。
- 解けた数: def 171 / cand1112 172。
- 対の比 (どちらかが解けた 44 問): **−5.4% [−9.5%, −1.3%]** (cand1111 は −2.5% [−6.8%, +1.6%])。
- 種をまたいで揃って遅い問題 (全種で比 > 1.25): なし。
- 最大 RSS: def 1861 MB / cand1112 1321 MB。
- 判定: P は採用の基準を満たす。M (Mittelmann) の M39〜M49 の結果待ち。
