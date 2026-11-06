# mushroom-best の再スタートの後の模型で HiGHS が出す z の式のカットの出どころと、ENOMOTO への試作 `ENOMOTO_MIP_MUSHAGG` (2026-11-06)

bench に使う環境変数: (途中。計測が済んだら書く)

(書きかけ。計測の途中でコミットしている)

## 1. HiGHS のカットの出どころ (事実)

- 計装: HiGHS v1.15.1 (github の ERGO-Code/HiGHS のタグ) に既存の `scripts/highs_instrument.patch` と、この調べの印付け
  (`mushroom_cuts_20261106/highs_tag.patch` に両方を含む差分) を当ててビルドした highs-bin。`HIGHS_POOL_TAG=ファイル` で、カットプールに
  入る全てのカットに「出どころ (tab / path / modk / impl / clq / cf-search / cf-heur)・方法 (CMIR / lifted cover の種類)・集約した LP の行と重み」
  を、LP に入れるときに `L` 行を書く。`HIGHS_GC_TRACE_MODK=ファイル` で mod-k の generateCut の途中 (置き換え・前処理・cover・CMIR) を書く。
- 模型: `ENOMOTO_MIP_DEBUG_DUMP_RESTART` で書き出した再スタート 1 回目の模型 (9197 行 × 8468 列、置いていない)。HiGHS は 4 スレッド・ノード上限 1・
  `objective_bound = 0.0553338` (`highs/root.opt`)。根の下界 0.0158 → **0.030521** (前の記録の 0.0305207 と同じ)。

| 設定 | 根の下界 |
|---|---|
| すべて (`none`) | 0.030521 |
| tableau を止める (`HIGHS_SKIP_SEP=0`) | 0.027454 |
| 経路集約を止める (`=1`) | 0.031115 |
| **mod-k を止める (`=2`)** | **0.015844** (ENOMOTO と同じ) |
| mod-k だけ (`=01`) | 0.031122 |
| 含意・クリークを止める | 0.030521 |

- LP に入ったカット 59 本 (根のループ全体の延べ) のうち 55 本が mod-k (lifted mixed-integer cover 39 本・CMIR 16 本)、4 本が経路集約。
  プールの z だけの式 835 本のうち 832 本が mod-k (`highs/tagsumm_root.txt`)。
- LP に入った mod-k のカットの集約は、**どれもデータの行 2 本 (x0 の符号が逆) + 0〜10 本の w-z の 2 項の行** (4 本のデータの行が 2 本)。
