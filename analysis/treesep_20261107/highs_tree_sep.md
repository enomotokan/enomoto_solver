# HiGHS v1.15.1 の木の中の分離と LP の行の年齢 (ソースを読んだまとめ)

パスは highs/mip/ から。版は v1.15.1 (04024d7)。

## 事実
- 分離するノード: 主ループ (HighsMipSolver.cpp:889-927) の processNode (:826-837) で evaluateNode → pruneNode → separateAndStoreBasis (:684-694)。
  `mip_allow_cut_separation_at_nodes` (既定 true) なら `sepa.separate(localdom)`。待ち行列から取り出したプランジの最初のノードだけ。
  潜り (HighsSearch::dive) の途中では分離しない。
- HighsSeparation::separate (HighsSeparation.cpp:181-227): ラウンド数の上限なし。続ける条件は `obj - rootlpsolobj > 1.01 * max(lastobj - rootlpsolobj, feastol)`
  (根の LP の値からの伸びが前のラウンドより 1% 超増える間)。カット 0 本・LP 非最適・小数の変数なしで止まる。時間・反復の上限なし。
- 1 ラウンド (separationRound :42-179): implied bound, clique, tableau/path/mod-k の分離器はプールに入れるだけ。プールの separate (:154)
  で選ばれたものだけが LP に入り、addCuts → resolveLp → `lp->performAging(true)`。
- LP の行の年齢 (HighsLpRelaxation.cpp:629-677): `performAging(true)` は分離のラウンドごと。agelimit = mip_lp_age_limit (既定 10)。
  `epochs` (LP 通算) を進め、`epochs % max(agelimit/2, 2) != 0` なら削除しない (既定 5 回に 1 回削除)。`epochs < agelimit` の間は上限を epochs に下げる。
  基底 (余裕あり、退化を含む) の行は年齢 +1、`age > agelimit` で削除してプールへ戻す (lpCutRemoved: 年齢 1)。非基底で双対値が 0 でない行は 0 に戻す。
  evaluateNode (HighsSearch.cpp:958) の `performAging(false)` は年齢が 0 でない行だけ +1 し、削除しない。
- removeObsoleteRows (:536-557) は根とヒューリスティクスだけ。LP の行の数の上限は見当たらない。
- プール (HighsCutPool.cpp): LP 外のカットは破られないと年齢 +1、`mip_pool_age_limit` (30) で削除。LP 外のカットが soft limit (10000) を超えると上限を下げる。
- 根のカットと木で加えたカットの区別はない (並列のときワーカーごとのプールがあるだけ)。

## 推測
- 年齢 10 で外すので、木の LP は「最近の分離で効いたカット」だけになる。根のカットも同じ扱いで、外れたものはプールから違反したら戻る。
