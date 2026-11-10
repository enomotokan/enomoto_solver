# PAPER5: 文脈を見るカット選択 (SCIP の cutsel_ensemble) の試作 (2026-11-10)

bench に使う環境変数: `ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=s` (s = 1, 2) に、設定ごとに
def = なし (既定 = cand1108zd)、ens = `ENOMOTO_MIP_CUTSEL_ENS=1`、ensw = `ENOMOTO_MIP_CUTSEL_ENS=w`、ensk = `=k`、ensd = `=d`、enskd = `=kd`。
計測のスクリプトは `cutsel_ens_20261110/bench.py` (`EXTRA_CFG` で変種を足す)、根の表と矛盾の検査は `summ.py`、生データは `cutsel_ens_20261110/*.jsonl`・`logs/`。
機械は 4 CPU、ソルバーは同時に 1 つ。

## 1. 結論 (先に)

__RESULT__

## 2. 文献と SCIP のソース

- Turner・Berthold・Besançon "A Context-Aware Cutting Plane Selection Algorithm for Mixed-Integer Programming" (ZIB Report 23-21・arXiv 2307.07322 (2023)、
  Operations Research Proceedings 2023 (2025))。要旨 (WebSearch の抜粋): 点数の項・間引き・止め方の 3 つに、カットそのものと分離の文脈 (擬似費用など) の
  情報を入れる。密度で捨てる間引きを含む。SCIP 8.0.3・MIPLIB 2017 benchmark・種 1〜5 で 5% 速い。本文の表 (SMAC で調整した重みの値) は
  arxiv.org に届かず**未確認**。
- SCIP のソース `src/scip/cutsel_ensemble.c` (GitHub の master、2026-11-10 に取得) で確かめた事実:
  - 既定の重み: 効き目 0.75、directed cutoff distance 0、目的との平行度 0.25 (**既定では平行度を罰する** `penaliseobjparal = TRUE`、点は 0.25 × (1 − 平行度))、
    整数台 0.45、期待される改善 (効き目 × 平行度) 0.1、擬似費用 0.75 (木だけ)、ロック 0.25 (**既定ではロックの少ないものを良しとする**)、
    疎の点の最大 0.5 (密度 0.2 で 0 になる直線)、数値の点 0。効き目・期待される改善・dcd は (log1p(v) / log1p(最大))^2 で正規化。
  - 間引き: 密度 (非零の数 / LP の列数) が 0.425 を超えるものを先に捨てる。採ったカットと平行度 0.95 を超える残りは点から 0.25 引く (1 なら捨てる)。
    `filterparalcuts` (平行なものを捨てる) は既定で偽。
  - 止め方: 点の下限 0 (相対の閾値はない)、非零の予算は根で 4.5 × LP の列数 (木 9.5)、予算の残りが 1 列ぶん未満になったら残りに収まらない密度のものを捨てる。
    `maxcuts` = 200 (候補がこれを超えると ensemble は選ばず次の選び方に任せる)、変数 50000 まで。
  - **優先度は 7000 で、cutsel_hybrid (8000) より低い**。つまり SCIP の既定ではこの選び方は使われていない (ソースで確認。論文の「次の版に入れる」は
    「入っているが既定ではない」の形で実現している)。

## 3. 実装 (`src/mip/cutsel_ens.rs`、既定では使わない)

- `ENOMOTO_MIP_CUTSEL_ENS=<文字>` (値 `1` で上の SCIP の既定どおり)。外側の根のカットのループ (主・裏のカットの写し) と、根の選び方の比べ
  (`ENOMOTO_MIP_CUTSEL_PROBE`) の既定の側で、`select_cuts` (旧 cutsel_hybrid 相当: 効き目 + 0.1 × 目的との平行度 + 0.1 × 整数台、最良の 0.5 倍未満を捨て、
  平行度 0.3 で捨てる) の代わりに使う。固定費用のネットワーク型 (`fc_relax`) とサブ MIP・木のノードでは使わない (既定のまま)。
- 項: 効き目・期待される改善・目的との平行度 (罰)・整数台・密度の点・ロック (元の行から数えた上下のロック。カットは `a x <= b` なので a > 0 は上、a < 0 は下)。
  擬似費用の項は根では SCIP でも 0 なので入れていない。dcd と数値の点は SCIP の既定で重み 0 なので入れていない。乱数の 1e-6 も入れていない (同点は番号の小さい方)。
- 選択を通さずに入るカット (implied bound の `forced`) で残りを先に罰する (SCIP の forced cuts と同じ)。
- 本数の上限は今の上限 (根 50 本、伸びないラウンドで 100・200… に広げる `CUT_WIDEN`)。文字 `w` で 2000 (SCIP の `separating/maxcutsroot`)。
- 変種の文字: `k` = 本数を既定の選び方が採る本数にそろえる (並べ方だけ ensemble)、`d` = 密度で捨てず密度の点・非零の予算も使わない、
  `f` = 平行なものを罰でなく捨てる、`c` = 候補が 200 本を超えたら既定に戻す (SCIP の `maxcuts`)、`p` = 平行度の罰の値を変数で変える。
- 単体テスト 2 本 (平行なカットを捨てる・密なカットを捨てる)。

__BODY__
