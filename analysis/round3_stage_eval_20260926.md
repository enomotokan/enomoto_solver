# cont1 修正 (段階 1) と nug08-3rd / irish-electricity 対応 (段階 2) の判定

## 段階 1 (a666575): 採用 (正しさの修正)
依頼者側で base = de3eb33 と比較。

- cont1: optimal、0.008782486003705103 (HiGHS 0.00878248600370569 と相対 7e-14)、335.9 s。
  以前は元問題の 12 行を最大 6.6e-3 違反する実行不能解を optimal として返していた。
- Netlib93 (`ab_bench --rounds 3`): status 全一致、目的関数値の相対差 最大 5.6e-14、合計 −0.90%、幾何平均 −0.17%、10% 超の退行なし (最大 pilot.ja +7.8%)。
- Mittelmann 5 問の目的関数値は不変 (stormG2_1000 59.2 s, square41 90.3 s, pds-100 105.1 s, ex10 86.8 s, s250r10 69.6 s)。

## 段階 2 (91c1c3d, 06cb0fd, 474bf5a): 不採用 → 取り消し
実装担当の計測 (段階 1 後との比較、Netlib93 + Mittelmann 11 問):
- nug08-3rd 499 → 326 s (−34.7%)。他は ±3.4% 以内 (fome13 +3.4%)、irish-electricity は打ち切りのまま。
- Netlib93 はビット一致。104 問の shifted geomean (shift 100 ms) −0.6% で、採用基準 −2% に届かない。
- 内容: LU 稠密切替の自動有効化 (m ≥ 1 万 かつ nnz(LU) ≥ 32m)、Markowitz 消去による従属等式の除去 (従属行が等式の 1% 以上のとき)。
  irish-electricity 向けの ineqsingleton 既定化・特異基底からの復旧・許容値統一は、完走に至らず取り下げ。
- 取り消しは作業ツリーを a666575 の src に戻した 1 コミット。実装は履歴 (上記 3 コミット) と docs/improvement_history.md の履歴から再利用できる。
