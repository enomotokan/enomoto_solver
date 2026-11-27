
## root_long と root_def

| 問題 | 集合 | c root_def | c root_long | 差 | 手間 root_def / root_long (1e9) | c@ (root_def を root_long の手間で) | ラウンド | 理由 root_def / root_long |
|---|---|---|---|---|---|---|---|---|
| 50v-10 | M | 74.62% | 82.07% | 7.45% | 1.65 / 6.91 | 74.62% | 25 / 90 | round_limit / phase2_stall |
| academictimetablesmall | M | - | - | - | 25.55 / 25.55 | - | 12 / 12 | stall / stall |
| b1c1s1 | M | 78.52% | 79.63% | 1.11% | 13.29 / 15.48 | 78.52% | 85 / 100 | round_limit / round_limit |
| beasleyC3 | M | 93.44% | 99.26% | 5.83% | 0.39 / 2.47 | 93.44% | 25 / 100 | round_limit / round_limit |
| bnatt500 | M | - | - | - | 2.17 / 2.17 | - | 11 / 11 | stall / stall |
| cbs-cta | M | - | - | - | 3.01 / 3.01 | - | 5 / 5 | stall / stall |
| cmflsp50-24-8-8 | M | 3.61% | 3.66% | 0.05% | 95.39 / 127.89 | 3.61% | 12 / 13 | time_cap / stall |
| cost266-UUE | M | 35.69% | 38.65% | 2.96% | 11.90 / 21.52 | 35.69% | 25 / 69 | round_limit / phase2_stall |
| cryptanalysiskb128n5obj14 | M | - | - | - | 80.06 / 99.32 | - | 11 / 14 | time_cap / stall |
| cryptanalysiskb128n5obj16 | M | - | - | - | 98.25 / 79.79 | - | 13 / 13 | stall / stall |
| dws008-01 | M | 12.43% | 12.77% | 0.34% | 7.49 / 11.31 | 12.43% | 84 / 100 | phase2_time / round_limit |
| fhnw-binpack4-4 | M | - | - | - | 0.27 / 0.27 | - | 11 / 11 | stall / stall |
| glass-sc | M | 2.09% | 2.17% | 0.08% | 2.01 / 2.69 | 2.09% | 25 / 38 | round_limit / phase2_stall |
| leo2 | M | 26.92% | 31.10% | 4.18% | 12.21 / 17.05 | 26.92% | 25 / 52 | round_limit / phase2_stall |
| lotsize | M | 89.59% | 91.92% | 2.33% | 5.52 / 27.92 | 89.59% | 69 / 100 | phase2_time / round_limit |
| milo-v12-6-r2-40-1 | M | 69.66% | 85.65% | 15.99% | 3.90 / 11.36 | 69.66% | 25 / 85 | round_limit / phase2_stall |
| n3div36 | M | 64.02% | 64.40% | 0.39% | 11.44 / 17.09 | - | 50 / 67 | round_limit / stall |
| n5-3 | M | 91.98% | 92.50% | 0.53% | 13.57 / 15.96 | 91.98% | 85 / 100 | round_limit / round_limit |
| neos-1122047 | M | - | - | - | 0.00 / 0.00 | - | 0 / 0 | gap_closed / gap_closed |
| neos-1171448 | M | - | - | - | 11.44 / 11.44 | - | 8 / 8 | stall / stall |
| neos-1171737 | M | - | - | - | 4.15 / 4.15 | - | 7 / 7 | stall / stall |
| neos-1445765 | M | 55.04% | 65.50% | 10.46% | 3.59 / 6.65 | 55.04% | 25 / 76 | round_limit / phase2_stall |
| neos-1582420 | M | 49.59% | 48.49% | -1.10% | 8.70 / 7.63 | - | 57 / 46 | stall / phase2_stall |
| neos-3024952-loue | M | 52.30% | 68.67% | 16.37% | 2.32 / 5.15 | 52.30% | 25 / 66 | round_limit / phase2_stall |
| neos-3216931-puriri | M | 0.68% | 0.68% | 0.00% | 177.55 / 261.74 | 0.68% | 9 / 11 | time_cap / time_cap |
| neos-3656078-kumeu | M | 7.57% | 9.88% | 2.31% | 273.47 / 242.41 | 7.57% | 6 / 6 | lp_fail / lp_fail |
| neos-3988577-wolgan | M | - | - | - | 82.05 / 91.11 | - | 6 / 7 | stall / stall |
| neos-4387871-tavua | M | 54.51% | 63.38% | 8.87% | 1.69 / 7.76 | 54.51% | 58 / 100 | phase2_time / round_limit |
| neos-4954672-berkel | M | 34.08% | 55.87% | 21.79% | 1.84 / 5.34 | 34.08% | 25 / 100 | round_limit / round_limit |
| neos-873061 | M | 49.71% | 49.94% | 0.24% | 63.66 / 194.61 | 49.71% | 9 / 11 | time_cap / time_cap |
| neos-933966 | M | - | - | - | 9.61 / 9.61 | - | 7 / 7 | stall / stall |
| neos-950242 | M | - | - | - | 0.01 / 0.01 | - | 0 / 0 | gap_closed / gap_closed |
| neos859080 | M | - | - | - | 0.02 / 0.02 | - | 5 / 5 | stall / stall |
| net12 | M | 7.61% | 12.82% | 5.21% | 80.65 / 149.31 | 7.61% | 14 / 86 | time_cap / intobj_stall |
| nexp-150-20-8-5 | M | 18.93% | 23.10% | 4.17% | 22.05 / 27.27 | 18.93% | 85 / 100 | round_limit / round_limit |
| ns1830653 | M | 29.48% | 29.99% | 0.51% | 6.77 / 12.09 | 29.48% | 25 / 58 | round_limit / phase2_stall |
| ns1952667 | M | - | - | - | 4.99 / 4.99 | - | 7 / 7 | stall / stall |
| peg-solitaire-a3 | M | - | - | - | 3.30 / 3.30 | - | 6 / 6 | stall / stall |
| radiationm40-10-02 | M | 34.46% | 50.51% | 16.06% | 71.60 / 108.75 | 34.46% | 10 / 13 | time_cap / stall |
| rail01 | M | 53.56% | 44.78% | -8.78% | 252.15 / 254.81 | 53.56% | 9 / 9 | time_cap / time_cap |
| rococoC10-001000 | M | 62.24% | 69.45% | 7.21% | 0.54 / 1.29 | 62.24% | 25 / 83 | round_limit / phase2_stall |
| sing44 | M | 53.15% | 54.41% | 1.25% | 66.40 / 70.25 | 53.15% | 17 / 18 | time_cap / stall |
| sorrell3 | M | 11.52% | 17.98% | 6.46% | 95.99 / 186.58 | 11.52% | 9 / 71 | time_cap / phase2_stall |
| tbfp-network | M | 33.90% | 34.29% | 0.39% | 119.37 / 190.19 | - | 11 / 22 | lp_fail / stall |
| thor50dday | M | 68.14% | 81.17% | 13.03% | 67.90 / 214.61 | 68.14% | 14 / 56 | time_cap / lp_fail |
| uct-subprob | M | 19.59% | 25.24% | 5.66% | 4.74 / 6.66 | 19.59% | 25 / 59 | round_limit / phase2_stall |
| assign1-5-8 | P | 7.91% | 9.05% | 1.14% | 1.06 / 1.84 | 7.91% | 25 / 45 | round_limit / phase2_stall |
| beavma | P | 100.00% | 100.13% | 0.13% | 0.15 / 0.15 | - | 37 / 35 | lp_integral / gap_closed |
| blp-ar98 | P | 42.70% | 54.24% | 11.54% | 6.84 / 18.73 | 42.70% | 13 / 38 | time_cap / time_cap |
| cvs16r128-89 | P | 9.67% | 10.65% | 0.98% | 7.07 / 10.39 | 9.67% | 16 / 29 | time_cap / no_cands |
| drayage-100-23 | P | - | - | - | 0.07 / 0.07 | - | 7 / 7 | stall / stall |
| exp-1-500-5-5 | P | 99.72% | 99.97% | 0.25% | 1.90 / 2.29 | 99.72% | 85 / 100 | round_limit / round_limit |
| h80x6320d | P | 96.68% | 97.26% | 0.58% | 9.03 / 22.32 | - | 50 / 59 | stall / stall |
| mas74 | P | 8.41% | 8.65% | 0.24% | 0.11 / 0.14 | - | 30 / 41 | stall / stall |
| mc11 | P | 98.50% | 99.05% | 0.55% | 2.83 / 5.96 | 98.50% | 72 / 126 | phase2_time / phase2_stall |
| mik-250-20-75-4 | P | 82.06% | 84.07% | 2.02% | 0.50 / 0.67 | - | 50 / 81 | round_limit / stall |
| neos-1456979 | P | 31.75% | 59.09% | 27.34% | 2.44 / 6.08 | 31.75% | 25 / 46 | round_limit / phase2_stall |
| neos-827175 | P | - | - | - | 7.08 / 7.08 | - | 8 / 8 | lp_integral / lp_integral |
| neos-860300 | P | 99.91% | 99.95% | 0.04% | 7.14 / 12.90 | - | 92 / 148 | gap_closed / gap_closed |
| neos-960392 | P | - | - | - | 0.52 / 0.52 | - | 10 / 10 | stall / stall |
| neos17 | P | 80.82% | 95.11% | 14.29% | 0.79 / 2.17 | 80.82% | 25 / 84 | round_limit / no_cands |
| neos5 | P | 22.40% | 25.70% | 3.30% | 0.19 / 0.70 | 22.40% | 25 / 85 | round_limit / phase2_stall |
| ns1116954 | P | - | - | - | 17.89 / 17.89 | - | 5 / 5 | stall / stall |
| nu25-pr12 | P | 99.67% | 100.00% | 0.33% | 0.84 / 0.98 | - | 32 / 48 | stall / gap_closed |
| p0201 | P | 40.88% | 41.69% | 0.81% | 0.14 / 0.17 | 40.88% | 25 / 40 | round_limit / phase2_stall |
| p200x1188c | P | 83.55% | 89.14% | 5.59% | 1.78 / 2.47 | - | 117 / 182 | round_limit / no_cands |
| qnet1 | P | 95.05% | 90.53% | -4.52% | 2.05 / 1.51 | - | 90 / 76 | stall / phase2_stall |
| roll3000 | P | 72.57% | 80.25% | 7.69% | 2.52 / 10.06 | 72.57% | 25 / 84 | round_limit / phase2_stall |
| supportcase26 | P | 15.07% | 18.98% | 3.92% | 0.40 / 2.29 | 15.07% | 25 / 79 | round_limit / phase2_stall |
| timtab1 | P | 73.64% | 74.67% | 1.03% | 1.05 / 1.22 | 73.64% | 85 / 100 | round_limit / round_limit |

- 閉じた割合の差 (root_long − root_def) 10/25/50/75/90%: 0 / 0 / 0 / 0.000957 / 0.0418 (n=177、平均 0.0129)。0.1% 超で良い 44 問・悪い 3 問
- 手間の比 (root_long / root_def): 1 / 1 / 1 / 1.06 / 1.85 (n=196、平均 1.29)、幾何平均 1.173
- 同じ手間での差 (c root_long − c@ root_def): 0 / 0 / 0 / 0.000514 / 0.0566 (n=150、平均 0.015)。良い 36 問・悪い 1 問
- 表は閉じた割合が 0.1% 超変わるか手間が 2 割超変わった問題だけ (70 問)
