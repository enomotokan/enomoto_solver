# PAPER7: 完全単模 (TU) の部分による暗黙の整数 (2026-11-10、途中)

bench に使う環境変数: `ENOMOTO_MIP_IMPLINT_TU2` (試作、既定では使わない。値の文字で部品を選ぶ。`1` は `b` と同じ)、
`ENOMOTO_MIP_IMPLINT_TU` (既存の試作)、`ENOMOTO_MIP_OBJ_STEP_IMPLINT` (既存の試作、目的値の刻みに暗黙の整数を使う)、
診断 `ENOMOTO_MIP_DEBUG_IMPLINTCOUNT=stop` (前処理の後の問題で判定ごとの数を出して終える)。
計測の設定の名前 (`implint_tu_20261110/run.py` の `CFGS`): `def` (なし)、`tu` (`IMPLINT_TU=1`)、`tu2` (`TU2=1`)、`tu2a` (`TU2=ba`)、
`tu2as` (`TU2=bas`)、`tu2ns` (`TU2=bns`)。

(書きかけ。結論と数は計測が終わってから書く)

## 文献

- van der Hulst・Walter「Implied Integrality in Mixed-Integer Optimization」(IPCO 2025、arXiv 2504.07209)。この環境から arxiv.org・
  optimization-online.org に届かず本文は読めていない (検索の抜粋だけ)。抜粋: 整数列の部分を整数に固定した残りの多面体が整数なら暗黙の
  整数、TU の部分行列で判定、前処理の後に整数性を強制する列の割合が 70.2% → 59.0% (版によって 70.8% → 60.5%)。
- SCIP の `presol_implint.c` (GitHub の scipopt/scip master、2025 年) は読めた: 連続列の部分行列の連結成分ごとに、行の連続列の係数が全部 ±1、
  行の整数列の係数・左辺・右辺が整数、列の境界が整数なら、成分がネットワーク行列か転置ネットワーク行列か (`SCIPnetmatdecTryAddCol`・
  `TryAddRow`、SPQR 木による増分の認識) を調べ、そうなら成分の列を丸ごと「弱い暗黙の整数」にする。行の倍率はかけない。
  対称性の処理の前に走らせる (注釈: 対称性の扱いは軌道の列がどれも整数であることを前提にすることがある)。
- SCIP の `cuts.c`・`sepa_aggregation.c` (同じ master): 連続の暗黙の整数は c-MIR で変数上下限の置き換えを許し (`vartypeusevbds` の既定 2 が
  「連続」と「連続の暗黙の整数」の区分を含む)、置き換えなければ整数として丸める。集約では連続として消す (`IMPLINTSARECONT`)。
  SCIP 10 の公開版でも同じかは未確認。
