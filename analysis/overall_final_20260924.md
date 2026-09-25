# 全体非効率対策 (第 2 弾) の最終結果 (2026-09-24)

分析 (Fable): `lu_core_20260924_022917.md` / `simplex_loop_20260924_113533.md` / `presolve_pipeline_20260924_031005.md`。
実装 (Opus): 各担当ブランチをマージ (LU×2、単体法ループ×2、polish/handoff、presolve×2)。
採用基準: 全 93 問で幾何平均が改善し、10% を超える退行が無いこと。

## 最終 A/B (base = 8cc4303, new = 9ec0ea1, 全 93 問, 3 rounds, `scripts/ab_bench.py`)
生データ: `overall_final_ab_20260924.json`

```
problem         base       new   change
pilot         0.8839    0.8884    +0.5%
scsd8         0.0514    0.0502    -2.4%
greenbeb      0.4275    0.4100    -4.1%
25fv47        0.2114    0.2024    -4.2%
grow22        0.1249    0.1187    -5.0%
perold        0.0626    0.0586    -6.3%
pilot.we      0.1210    0.1114    -7.9%
fit1p         0.0537    0.0494    -8.0%
bnl1          0.0335    0.0307    -8.2%
pilot.ja      0.1396    0.1273    -8.8%
boeing1       0.0086    0.0078    -9.1%
stair         0.0114    0.0101   -11.0%
d2q06c        1.3076    1.1562   -11.6%
dfl001        8.1537    7.1586   -12.2%
pilotnov      0.0911    0.0797   -12.5%
maros         0.0336    0.0291   -13.3%
e226          0.0057    0.0049   -13.4%
pilot4        0.0287    0.0248   -13.4%
greenbea      0.1743    0.1507   -13.5%
sc205         0.0019    0.0017   -14.1%
grow15        0.0490    0.0420   -14.4%
degen2        0.0171    0.0146   -14.9%
fit2p         1.5514    1.3116   -15.5%
etamacro      0.0083    0.0069   -17.1%
degen3        0.2206    0.1828   -17.1%
bandm         0.0068    0.0056   -17.4%
grow7         0.0113    0.0091   -19.2%
scsd6         0.0085    0.0068   -19.4%
scfxm1        0.0058    0.0046   -19.6%
share2b       0.0012    0.0010   -19.7%
bnl2          0.0705    0.0566   -19.8%
scfxm3        0.0305    0.0245   -19.8%
israel        0.0020    0.0016   -20.0%
d6cube        0.0899    0.0718   -20.1%
modszk1       0.0213    0.0169   -20.9%
adlittle      0.0009    0.0007   -21.0%
nesm          0.0822    0.0647   -21.3%
scfxm2        0.0161    0.0125   -22.1%
sctap1        0.0047    0.0037   -22.3%
boeing2       0.0018    0.0014   -22.5%
agg3          0.0039    0.0030   -22.7%
finnis        0.0051    0.0040   -22.8%
gfrd-pnc      0.0062    0.0048   -22.9%
agg2          0.0036    0.0027   -24.4%
pilot87       4.9688    3.7434   -24.7%
sc105         0.0007    0.0005   -24.8%
80bau3b       0.2134    0.1584   -25.8%
shell         0.0071    0.0053   -25.8%
maros-r7      1.0312    0.7644   -25.9%
blend         0.0009    0.0007   -25.9%
scagr25       0.0050    0.0037   -26.0%
forplan       0.0069    0.0050   -27.5%
sctap2        0.0134    0.0097   -27.5%
agg           0.0023    0.0017   -27.5%
capri         0.0030    0.0022   -27.7%
ship04l       0.0096    0.0069   -27.8%
ship08s       0.0112    0.0081   -27.9%
scrs8         0.0074    0.0053   -28.9%
czprob        0.0204    0.0143   -30.2%
sctap3        0.0262    0.0183   -30.3%
fit1d         0.0064    0.0044   -30.3%
stocfor2      0.0307    0.0213   -30.7%
ship12s       0.0139    0.0095   -31.7%
cycle         0.0313    0.0213   -31.9%
ship04s       0.0065    0.0044   -32.0%
ship08l       0.0232    0.0157   -32.6%
kb2           0.0003    0.0002   -33.0%
brandy        0.0049    0.0033   -33.0%
fffff800      0.0092    0.0061   -33.4%
seba          0.0046    0.0031   -33.5%
fit2d         0.1124    0.0746   -33.6%
tuff          0.0059    0.0039   -33.7%
ship12l       0.0336    0.0221   -34.5%
ganges        0.0137    0.0085   -38.0%
sc50a         0.0002    0.0001   -39.3%
stocfor1      0.0009    0.0006   -40.8%
sierra        0.0158    0.0093   -40.8%
share1b       0.0023    0.0014   -41.1%
scorpion      0.0018    0.0010   -41.2%
sc50b         0.0002    0.0001   -41.6%
recipe        0.0007    0.0004   -42.2%
woodw         0.0593    0.0343   -42.2%
lotfi         0.0023    0.0013   -42.5%
standmps      0.0048    0.0027   -43.8%
scagr7        0.0010    0.0005   -46.9%
standgub      0.0039    0.0020   -47.6%
vtp.base      0.0010    0.0005   -48.1%
scsd1         0.0040    0.0021   -48.1%
standata      0.0036    0.0019   -48.5%
bore3d        0.0015    0.0008   -48.9%
afiro         0.0001    0.0001   -50.6%
wood1p        0.0935    0.0420   -55.1%
beaconfd      0.0022    0.0008   -64.7%
TOTAL base=20.968s new=17.573s change=-16.19%
GEOMEAN ratio new/base=0.7263 change=-27.37%
regressions >10%: []
```

全 93 問 optimal、目的関数値は base と一致 (相対 1e-6 以内)。

## 採用 (既定 on)
- presolve: 従属等式検出を重複行除去のみに (C1, 経路変更)。以下ビット同一: dedupe の u64 ハッシュ化 (C2)、aggregator v2 / doubleton / parallelcols / dualpropagate の空振り早期 return (C3/C9/C10/C11)、reduce_inequalities の単項行判定と容量確保 (C5/C14)、スケーリング平坦化 (C8)、extract_bounds 往復省略 (C12)、境界を行にしない split 形 (C13)、標準形の直接構築 (C15)、aggregator 列索引の一括確保 (C19)、固定変数畳み込みの省略、結果出力の軽量化 (C17)。
- LU (ビット同一): U/R eta の平坦化 `EtaFile` (C1)、τ FTRAN の L 段 GP 経路 (C3)、単位 BTRAN の O(m) パス削減 (C4)、L 段 permute 融合 (C6)、再分解固定費 (D1)、pack 1 パス化 (D2)、ピボット探索の rmin 打ち切り (B2a)、eliminate の in-place 書き直し、FTRAN U 段の hyper-sparse 化 (C5)。
- 単体法ループ (ビット同一): env 読み出しの OnceLock キャッシュ (S13)、x_B 更新・DSE 更新・PRICE の非ゼロ行リスト化 (S6/S7/S8)、PRICE swap の O(1) 化 (S12)、refresh_row 軽量化 (S10)、setup 構築削減 (S15)、初期 d 計算の省略 (S14)、polish 双対判定から固定列を除外 (S3)。
- パラメータ再調整 (経路変更; 組み合わせ A/B でマージ後既定比 幾何平均 -2.78%、合計 -11.6%、10% 超の退行なし):
  `XB_CHECK_INTERVAL` 5→20、`PIVOT_SEARCH_LIMIT` 256→64、primal handoff の増分化 + Devex (`ENOMOTO_HANDOFF_INCREMENTAL`/`_INC_DEVEX` 既定 1)。

## 不採用 (ノブとして残し既定 off / 旧値)
確認 A/B (マージ後ビルド、2 rounds、退行は 6 rounds で再計測) の要約:
```
ruiz3        gm=0.9592 tot=  -0.39% bad=0 reg>10%: [('etamacro', '+42.0%'), ('modszk1', '+26.6%'), ('greenbea', '+23.5%'), ('sctap2', '+19.8%'), ('blend', '+18.3%'), ('tuff', '+14.9%')]
xbint20      gm=0.9771 tot=  -6.32% bad=0 reg>10%: [('scagr7', '+15.9%'), ('afiro', '+14.9%')]
tiny13       gm=0.9800 tot=  +1.28% bad=0 reg>10%: [('pilot', '+36.6%'), ('pilot.we', '+12.9%')]
structstop   gm=0.9863 tot=  +1.03% bad=0 reg>10%: [('pilot', '+12.9%'), ('afiro', '+11.3%')]
xblist05     gm=0.9921 tot=  +0.35% bad=0 reg>10%: [('blend', '+18.7%')]
ho_devex     gm=0.9963 tot=  -1.98% bad=0 reg>10%: [('degen3', '+19.9%'), ('sc50a', '+16.6%'), ('pilot.ja', '+11.5%')]
search64     gm=0.9969 tot=  -4.67% bad=0 reg>10%: [('fit2d', '+20.7%')]
freshfloor3  gm=0.9970 tot=  -1.77% bad=0 reg>10%: [('kb2', '+17.5%'), ('pilot', '+11.0%')]
harris3e8    gm=0.9987 tot=  -4.04% bad=0 reg>10%: [('greenbea', '+27.0%'), ('sctap2', '+23.2%'), ('grow7', '+22.1%'), ('fit1d', '+20.4%'), ('pilot', '+14.8%'), ('maros', '+10.1%')]
dcol07       gm=0.9992 tot=  +0.03% bad=0 reg>10%: [('25fv47', '+17.1%'), ('fit2d', '+16.4%'), ('sc50a', '+10.9%')]
stab04       gm=1.0001 tot=  +4.85% bad=0 reg>10%: [('tuff', '+22.9%'), ('grow22', '+19.9%'), ('fit2d', '+12.9%'), ('pilot', '+11.8%')]
backoff2     gm=1.0054 tot=  +1.31% bad=0 reg>10%: [('80bau3b', '+19.8%'), ('stocfor1', '+14.8%'), ('scagr7', '+10.6%')]
davg002      gm=1.0077 tot=  -1.30% bad=0 reg>10%: [('blend', '+17.2%'), ('sc50a', '+13.3%'), ('80bau3b', '+12.8%')]
rowsearch2   gm=1.0080 tot=  -2.88% bad=0 reg>10%: [('afiro', '+77.2%'), ('sc105', '+21.9%'), ('pilot.we', '+16.3%'), ('pilot', '+14.1%')]
davg001      gm=1.0082 tot=  -1.85% bad=0 reg>10%: [('scagr7', '+11.3%'), ('vtp.base', '+10.1%')]
xbsample8    gm=1.0092 tot=  +0.26% bad=0 reg>10%: [('sc50a', '+13.3%')]
chuzr8       gm=1.0183 tot=  +0.64% bad=0 reg>10%: []
pricecol01   gm=1.0253 tot=  +6.18% bad=0 reg>10%: [('pilot', '+30.5%'), ('scsd6', '+20.1%'), ('sc50b', '+15.3%'), ('dfl001', '+13.7%'), ('scsd8', '+12.2%')]
```
- 再計測で退行が実在: `ENOMOTO_XB_DRIFT_FRESH_FLOOR=3` (pilot +11.5%)、`ENOMOTO_T_ROUND_STRUCT_STOP=1` (pilot +14%)、`ENOMOTO_TINY=1e-13` (pilot +41.5%)。
- 幾何平均が改善しない、または 10% 超の退行が多数: Ruiz 反復数、Harris 許容、密度移動平均係数、ピボット閾値、再利用バックオフ、列方式 PRICE (S9)、CHUZR ショートリスト (S11)、行バケット探索 (B2b)、ドリフト検査の行サブサンプル (S2)、ドリフト許容の相対床/更新下限 (A1/A2)、密 LU 切替 (B3)、境界フリップ handoff (S5)。
- スクリーニング (86 設定、3 並列・負荷下) の結果は `scratchpad` のみ。負荷下では小問題で ±20% のノイズがあり、採否判定には用いていない。

## 計測手順の変更
`scripts/ab_bench.py`: 幾何平均の出力、`--base-env/--new-env`、計測前の較正パス (破棄) を追加。
