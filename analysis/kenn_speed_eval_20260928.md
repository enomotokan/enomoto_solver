# Kennington の遅い問題の高速化 (作業 #10、924b529..74996c3) の判定: 採用

内容 (詳細は docs/improvement_history.md の作業 #10 節、調査は analysis/kennington_slow_20260928_030000.md):
A chuzr の遅延ヒープをプール長で切り替えて m < 10,000 にも、B 部分 tau の散布形式への作り直し (ビット一致)、
D aggregator のピボット比を均衡化前の係数でも判定、J DSE 重み更新ループから tunable! を外す、
F 超疎 U 段・R 段・BTRAN のヒープをビット集合の待ち行列に、M BFRT フリップ列 FTRAN の O(m) fill 削除 (J・F・M はビット一致)。

## 依頼者側の計測 (base = 924b529、new = 74996c3)
- Netlib93 (`ab_bench --rounds 3`、kenn_speed_ab_20260928.json): status 全問一致、合計 −0.10%、幾何平均 −0.85%。10% 超は greenbea のみ。
- Kennington 16 問 (base/new 交互に 3 回ずつ、中央値、kenn_speed_kenn16_20260928.log): 全問 optimal。
  pds-20 5.18 → 3.31 s (−36%)、ken-11 −33%、ken-18 8.92 → 7.74 s (−13%)、cre-d −12%、cre-a −8% など。
- Mittelmann 11 問 (1 回ずつ、cont1・neos は打ち切り 1,500 s、kenn_speed_mitt11_20260928.log): 全問 optimal、目的関数値は base と一致 (s250r10・fome13 は最終桁)。
  pds-100 118.6 → 89.5 s (−25%)、s250r10 −5%、他は ±2%。fome13 +12.7%。
- 120 問の shifted geomean (shift 100 ms、600 s 打ち切り扱い): **−2.69%**。
- 10% 超の遅れ: greenbea +23%、fome13 +12.7%。どちらも D で前処理の出力が変わり反復数が増えたもの (greenbea 反復 +13%、fome13 +11.5%)。
  1 反復あたりの増加は前処理後の問題の形 (代入による fill-in、より密な基底を通る経路) によるもので、実装の作業量は増えていないことを実装担当が確認。
  方針 (反復経路の変化による時間の増減は本質的な退行とみなさない) により退行とみなさない。

判定: 基準を満たすので**採用**。
