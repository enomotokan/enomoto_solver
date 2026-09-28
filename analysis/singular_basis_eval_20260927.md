# 悪条件・特異基底への耐性 (作業 #5、e35aed9..f100926) の判定: 採用

内容: 行の雑音判定 (M1)、比率テストのピボット雑音判定 (M1')、誤った Infeasible の防止 (M2、Farkas 証明の確認付き)、
特異な再分解からの巻き戻し (M3)、既定未満のピボット閾値の引き上げ (M5)、試験スクリプト scripts/singular_stress.py (M10)。
ゲートは sqrt(w_r) > 1e9 (Netlib で選ばれる行の最大は perold の 2.9e8)。詳細は docs/improvement_history.md の該当節。

## 依頼者側の確認 (base = 3177ed7、new = f100926)
- 既定設定の Netlib93: status・目的関数値のビットが 93 問すべて base と一致 (経路不変)。
- `scripts/singular_stress.py --jobs 3 --cases` (生出力 singular_stress_20260927.txt):
  - 9 設定 (base, s1〜s6, s8, s9) すべてで Netlib 93/93 が optimal かつ HiGHS と一致。
  - irish-electricity (旧前処理): optimal、379 s、2546254.5633151024 (HiGHS と相対 2.3e-12)、元問題の最大違反 行 3.4e-7 / 列 5.7e-8。
    (base は 575 s で NotSolved。既定の前処理で解くと違反 1.3e-9。1e-7 超の 10 行はいずれもほぼ 0 の値を含む 2〜5 項の `≤ 0` 行。)
  - pilot87 (閾値 1e-3): optimal、3.8 s、301.7103473331112 (HiGHS と相対 1.2e-14)。base は NotSolved。

## 実装担当の計測
- Mittelmann 11 問: 経路不変 (目的関数値のビット一致)、時間差 ±3% は揺れ。
- 実行不能な合成問題 33 問: 32 問を証明付きで infeasible。pilot.ja 由来の 1 問は base の infeasible → NotSolved (HiGHS も Unknown。証明のない infeasible を返さなくなった方向の変化)。

判定: 頑健さの修正として**採用**。既定設定の経路を変えないので速度の基準は対象外。
