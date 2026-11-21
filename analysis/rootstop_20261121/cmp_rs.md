
## root_rs と root_def

| 問題 | 集合 | c root_def | c root_rs | 差 | 手間 root_def / root_rs (1e9) | c@ (root_def を root_rs の手間で) | ラウンド | 理由 root_def / root_rs |
|---|---|---|---|---|---|---|---|---|
| 50v-10 | M | 74.62% | 71.74% | -2.88% | 1.65 / 1.17 | 71.74% | 25 / 19 | round_limit / rootstop |
| academictimetablesmall | M | - | - | - | 25.55 / 25.55 | - | 12 / 12 | stall / stall |
| app1-2 | M | 88.61% | 88.36% | -0.26% | 35.60 / 29.98 | 88.36% | 83 / 65 | phase2_stall / rootstop |
| bnatt500 | M | - | - | - | 2.17 / 2.17 | - | 11 / 11 | stall / stall |
| cbs-cta | M | - | - | - | 3.01 / 3.01 | - | 5 / 5 | stall / stall |
| cryptanalysiskb128n5obj14 | M | - | - | - | 80.06 / 80.06 | - | 11 / 11 | time_cap / time_cap |
| cryptanalysiskb128n5obj16 | M | - | - | - | 98.25 / 98.25 | - | 13 / 13 | stall / stall |
| fhnw-binpack4-4 | M | - | - | - | 0.27 / 0.27 | - | 11 / 11 | stall / stall |
| icir97_tension | M | 58.38% | 56.79% | -1.59% | 3.29 / 1.63 | 56.79% | 19 / 8 | stall / rootstop |
| irp | M | 100.00% | 100.00% | 0.00% | 6.76 / 3.35 | - | 32 / 22 | lp_integral / lp_integral |
| lotsize | M | 89.59% | 81.58% | -8.02% | 5.52 / 3.61 | 81.58% | 69 / 52 | phase2_time / rootstop |
| mushroom-best | M | 50.05% | 47.41% | -2.64% | 10.11 / 8.12 | 47.41% | 39 / 22 | stall / rootstop |
| n5-3 | M | 91.98% | 88.57% | -3.41% | 13.57 / 7.33 | 88.57% | 85 / 45 | round_limit / rootstop |
| neos-1122047 | M | - | - | - | 0.00 / 0.00 | - | 0 / 0 | gap_closed / gap_closed |
| neos-1171448 | M | - | - | - | 11.44 / 11.44 | - | 8 / 8 | stall / stall |
| neos-1171737 | M | - | - | - | 4.15 / 4.15 | - | 7 / 7 | stall / stall |
| neos-3381206-awhea | M | 98.01% | 80.58% | -17.43% | 2.07 / 1.35 | 80.58% | 35 / 22 | intobj_stall / rootstop |
| neos-3656078-kumeu | M | 7.57% | 9.58% | 2.01% | 273.47 / 281.94 | 7.57% | 6 / 7 | lp_fail / lp_fail |
| neos-3988577-wolgan | M | - | - | - | 82.05 / 92.36 | - | 6 / 7 | stall / stall |
| neos-4738912-atrato | M | 89.27% | 89.16% | -0.11% | 1.92 / 1.59 | 89.16% | 21 / 18 | stall / rootstop |
| neos-5107597-kakapo | M | 0.03% | 0.03% | 0.00% | 5.85 / 4.19 | 0.03% | 14 / 9 | stall / rootstop |
| neos-5195221-niemur | M | 0.51% | 0.07% | -0.44% | 9.08 / 4.93 | 0.07% | 19 / 7 | stall / rootstop |
| neos-933966 | M | - | - | - | 9.61 / 9.02 | - | 7 / 6 | stall / rootstop |
| neos-950242 | M | - | - | - | 0.01 / 0.01 | - | 0 / 0 | gap_closed / gap_closed |
| neos859080 | M | - | - | - | 0.02 / 0.02 | - | 5 / 5 | stall / stall |
| ns1952667 | M | - | - | - | 4.99 / 4.99 | - | 7 / 7 | stall / stall |
| peg-solitaire-a3 | M | - | - | - | 3.30 / 3.30 | - | 6 / 6 | stall / stall |
| rail01 | M | 53.56% | 44.78% | -8.78% | 252.15 / 254.81 | 53.56% | 9 / 9 | time_cap / time_cap |
| rococoB10-011000 | M | 50.53% | 44.64% | -5.89% | 12.83 / 5.51 | 44.64% | 85 / 26 | phase2_stall / rootstop |
| rococoC10-001000 | M | 62.24% | 57.11% | -5.13% | 0.54 / 0.45 | 57.11% | 25 / 17 | round_limit / rootstop |
| snp-02-004-104 | M | 99.93% | 99.93% | -0.00% | 39.62 / 27.29 | 99.93% | 14 / 9 | stall / rootstop |
| tr12-30 | M | 99.56% | 93.93% | -5.63% | 2.19 / 0.50 | 93.93% | 71 / 19 | phase2_stall / rootstop |
| beavma | P | 100.00% | 98.48% | -1.52% | 0.15 / 0.08 | - | 37 / 17 | lp_integral / rootstop |
| drayage-100-23 | P | - | - | - | 0.07 / 0.07 | - | 7 / 7 | stall / stall |
| drayage-25-23 | P | 100.00% | 100.00% | 0.00% | 0.23 / 0.18 | - | 19 / 21 | gap_closed / gap_closed |
| eil33-2 | P | 48.22% | 47.65% | -0.57% | 1.91 / 1.64 | 47.65% | 15 / 10 | stall / rootstop |
| exp-1-500-5-5 | P | 99.72% | 96.35% | -3.36% | 1.90 / 1.15 | 96.35% | 85 / 54 | round_limit / rootstop |
| fiber | P | 97.40% | 94.50% | -2.90% | 0.65 / 0.24 | - | 124 / 57 | stall / stall |
| khb05250 | P | 99.90% | 99.78% | -0.12% | 0.48 / 0.33 | 99.78% | 11 / 6 | stall / rootstop |
| mas74 | P | 8.41% | 8.82% | 0.41% | 0.11 / 0.13 | - | 30 / 36 | stall / rootstop |
| mas76 | P | 13.36% | 12.40% | -0.96% | 0.08 / 0.08 | - | 24 / 25 | stall / stall |
| mc11 | P | 98.50% | 85.26% | -13.24% | 2.83 / 0.29 | 85.26% | 72 / 12 | phase2_time / rootstop |
| mik-250-20-75-4 | P | 82.06% | 81.74% | -0.31% | 0.50 / 0.34 | - | 50 / 32 | round_limit / rootstop |
| neos-827175 | P | - | - | - | 7.08 / 7.08 | - | 8 / 8 | lp_integral / lp_integral |
| neos-960392 | P | - | - | - | 0.52 / 0.37 | - | 10 / 7 | stall / rootstop |
| ns1116954 | P | - | - | - | 17.89 / 17.89 | - | 5 / 5 | stall / stall |
| p200x1188c | P | 83.55% | 91.53% | 7.98% | 1.78 / 0.99 | - | 117 / 80 | round_limit / no_cands |
| sp150x300d | P | 97.13% | 97.13% | 0.00% | 0.15 / 0.12 | - | 19 / 17 | gap_closed / gap_closed |
| timtab1 | P | 73.64% | 55.63% | -18.01% | 1.05 / 0.35 | 55.63% | 85 / 27 | round_limit / rootstop |

- 閉じた割合の差 (root_rs − root_def) 10/25/50/75/90%: -0.00315 / 0 / 0 / 0 / 0 (n=177、平均 -0.00527)。0.1% 超で良い 3 問・悪い 22 問
- 手間の比 (root_rs / root_def): 0.771 / 1 / 1 / 1 / 1 (n=196、平均 0.942)、幾何平均 0.920
- 同じ手間での差 (c root_rs − c@ root_def): 0 / 0 / 0 / 0 / 0 (n=150、平均 -0.00042)。良い 2 問・悪い 1 問
- 表は閉じた割合が 0.1% 超変わるか手間が 2 割超変わった問題だけ (49 問)
