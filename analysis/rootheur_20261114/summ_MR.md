| 問題 | 組 | 根のヒューリスティクスの始まり (秒) | 終わり | 主のスレッドの秒の和 | 1 秒以上使って改善なし (主) | 根の暫定解の出どころ (最後の 3 つ) | 結果 |
|---|---|---|---|---|---|---|---|
| neos-873061 | MR | 95.3 | 355.1 | 197.3 | vbounds (loose)=11.9 interior rounding=11.8 | LP face@268.8 root reduced cost@331.8 RINS@355.0 | ['node_limit', '119.3480173264'] |
| neos-3216931-puriri | MR | 123.5 | 305.0 | 176.4 | feasibility pump=68.0 fractional diving=48.2 vector length diving=31.3 coefficient diving=24.9 | locks@304.9 | ['node_limit', '81380.0'] |
| snp-02-004-104 | MR | 35.3 | 141.8 | 106.6 | interior rounding=16.3 | randomized rounding@35.6 RENS@92.2 RINS@141.8 | ['node_limit', '586876155.3559761'] |
| istanbul-no-cutoff | MR | 18.7 | 77.6 | 58.8 | RINS=26.7 randomized rounding=2.1 interior rounding=1.8 | int shifting@21.2 RENS@50.8 | ['node_limit', '207.83035499050843'] |
| hypothyroid-k1 | MR | 71.7 | 103.5 | 31.8 | interior rounding=27.2 RENS=4.0 | Local-MIP@4.4 Local-MIP@5.2 find thread@99.2 | ['node_limit', '-43.0'] |
| neos-933966 | MR | 138.2 | 167.8 | 29.4 | RINS=10.2 randomized rounding=1.9 interior rounding=1.8 | Local-MIP@121.3 shifting@138.8 RENS@157.4 | ['node_limit', '7385.0'] |
| supportcase40 | MR | 14.0 | 53.1 | 27.8 | RINS=18.1 RENS=7.8 interior rounding=1.1 | Local-MIP@7.5 root reduced cost@23.3 locks@31.1 | ['node_limit', '24537.29876220223'] |
| tbfp-network | MR | 149.2 | 177.6 | 27.5 | clique=23.7 interior rounding=2.0 | Local-MIP@140.1 locks@176.1 tree@0.0 | ['node_limit', '25.8888888886'] |
| fast0507 | MR | 95.2 | 123.1 | 26.8 | RINS=3.2 | interior rounding@98.4 RENS@118.6 LP face@118.8 | ['node_limit', '179.0'] |
| peg-solitaire-a3 | MR | 6.9 | 32.3 | 25.3 | feasibility pump=8.9 local search=5.0 vector length diving=4.3 fractional diving=3.0 |  | ['node_limit', 'None'] |
| net12 | MR | 41.4 | 64.0 | 22.6 | interior rounding=21.3 RENS=1.3 | Local-MIP@0.8 Local-MIP@24.9 | ['node_limit', '214.0'] |
| mzzv42z | MR | 26.1 | 46.1 | 19.6 | interior rounding=16.2 | Local-MIP@13.1 find thread@42.3 RENS@45.5 | ['node_limit', '-20520.0'] |
| unitcal_7 | MR | 21.5 | 39.2 | 17.3 | interior rounding=5.3 | Local-MIP@12.0 RINS@39.2 | ['node_limit', '19892644.4539975'] |
| neos-1456979 | MR | 0.8 | 19.1 | 11.9 | local search=5.0 feasibility pump=3.5 interior rounding=1.3 |  | ['node_limit', 'None'] |
| ran14x18-disj-8 | MR | 3.5 | 16.5 | 11.9 | RINS=5.7 | Local-MIP@1.4 Local-MIP@3.5 RENS@9.8 | ['node_limit', '3747.999999111189'] |
| satellites2-60-fs | MR | 98.0 | 113.2 | 11.8 | interior rounding=11.4 | Local-MIP@10.9 Local-MIP@18.2 locks@113.1 | ['node_limit', '19.0'] |
| CMS750_4 | MR | 4.8 | 27.6 | 11.6 | RENS=11.1 | Local-MIP@2.5 Local-MIP@3.0 Local-MIP@3.8 | ['node_limit', '252.0'] |
| mzzv11 | MR | 46.4 | 64.8 | 11.4 | interior rounding=9.6 RENS=1.3 | RENS@62.1 root reduced cost@64.7 RINS@64.8 | ['node_limit', '-21568.0'] |
| uccase12 | MR | 23.0 | 23.4 | 7.3 | RENS=1.2 | int shifting@4.1 interior rounding@8.6 LP face@9.9 | ['node_limit', '11507.408692634284'] |
| neos-2978193-inde | MR | 2.1 | 9.3 | 7.1 | RINS=3.2 | int shifting@2.3 randomized rounding@2.3 RENS@6.0 | ['node_limit', '-2.388061686'] |
| neos-4722843-widden | MR | 3.4 | 9.8 | 6.3 | interior rounding=2.1 | locks@8.8 vbounds (tight)@8.9 RINS@9.8 | ['node_limit', '25749.62376237624'] |
| neos-787933 | MR | 22.7 | 40.1 | 6.0 | RENS=1.2 interior rounding=1.1 | Local-MIP@3.2 RINS@40.1 tree@52.8 | ['node_limit', '37.0'] |
| irp | MR | 6.2 | 11.2 | 4.8 |  | tree@0.0 tree@0.0 tree@0.1 | ['optimal', '12159.492836'] |
| neos-662469 | MR | 8.0 | 43.5 | 4.5 | locks=2.6 | Local-MIP@3.9 find thread@9.3 clique@12.6 | ['node_limit', '834788.5'] |
| h80x6320d | MR | 4.0 | 6.6 | 3.4 | RINS=1.4 locks=1.2 | tree@0.0 LP face@4.6 tree@0.0 | ['node_limit', '6388.755906'] |
| cbs-cta | MR | 4.0 | 7.2 | 3.2 |  | Local-MIP@1.3 randomized rounding@4.4 RENS@7.2 | ['optimal', '-8.183503652503532e-09'] |
| tr12-30 | MR | 1.0 | 3.8 | 2.8 |  | Local-MIP@0.3 RENS@3.0 | ['node_limit', '131360.0'] |
| csched008 | MR | 0.6 | 5.2 | 2.7 | local search=1.4 |  | ['node_limit', 'None'] |
| sct2 | MR | 1.7 | 7.3 | 2.7 |  | randomized rounding@1.8 RENS@3.7 root reduced cost@6.6 | ['node_limit', '-228.1811031538334'] |
| supportcase33 | MR | 1.8 | 11.6 | 1.8 | interior rounding=1.5 | trivial@1.8 locks@4.6 | ['node_limit', '-185.0'] |
| roll3000 | MR | 1.6 | 5.9 | 1.6 |  | RENS@2.4 RINS@5.9 tree@7.5 | ['node_limit', '13291.000000000011'] |
| pg5_34 | MR | 1.7 | 3.2 | 1.5 |  | Local-MIP@0.9 randomized rounding@1.8 RENS@2.9 | ['node_limit', '-14297.335712007754'] |
| fiball | MR | 0.7 | 2.6 | 1.2 |  | Local-MIP@0.3 shifting@0.7 RENS@1.5 | ['node_limit', '147.0'] |
| neos-3627168-kasai | MR | 1.6 | 2.7 | 1.2 |  | Local-MIP@0.8 RENS@2.1 RINS@2.7 | ['node_limit', '996186.2999999999'] |
| rococoC10-001000 | MR | 0.4 | 1.6 | 0.9 |  | LP face@1.2 RINS@1.6 tree@1.8 | ['node_limit', '15054.0'] |
| traininstance6 | MR | 0.5 | 1.3 | 0.9 |  | shift-and-propagate@1.3 RINS@1.3 | ['node_limit', '41320.0'] |
| neos-4738912-atrato | MR | 3.1 | 4.5 | 0.8 |  | RENS@3.8 RINS@4.5 tree@4.6 | ['node_limit', '291573971.89946'] |
| mcsched | MR | 1.1 | 1.7 | 0.6 |  | Local-MIP@0.1 Local-MIP@0.8 RENS@1.7 | ['node_limit', '219825.99999999968'] |
| timtab1 | MR | 1.5 | 2.1 | 0.6 |  | Local-MIP@0.5 Local-MIP@0.8 RENS@2.1 | ['node_limit', '916549.9999997802'] |
| fastxgemm-n2r6s0t2 | MR | 0.2 | 0.5 | 0.4 |  | Local-MIP@0.0 Local-MIP@0.2 | ['node_limit', '532.9999999999999'] |
| mas74 | MR | 0.2 | 0.4 | 0.3 |  | LP face@0.4 root reduced cost@0.4 tree@0.0 | ['node_limit', '12640.37125820001'] |
| rocI-4-11 | MR | 3.7 | 4.0 | 0.3 |  | Local-MIP@3.2 | ['node_limit', '-2.0'] |
| mas76 | MR | 0.2 | 0.3 | 0.2 |  | RENS@0.3 root reduced cost@0.3 tree@0.0 | ['node_limit', '40005.05414200001'] |
| neos-960392 | MR | 0.8 | 1.0 | 0.2 |  | Local-MIP@0.3 Local-MIP@0.8 RENS@1.0 | ['optimal', '-238.0'] |
| neos5 | MR | 0.2 | 0.4 | 0.2 |  | tree@0.0 randomized rounding@0.2 RENS@0.4 | ['node_limit', '15.0'] |
| enlight_hard | MR | 0 | 0 | 0 |  |  |
| gen-ip054 | MR | 0.1 | 0.1 | 0.0 |  | Local-MIP@0.1 ZI round@0.1 RENS@0.1 | ['node_limit', '6979.373733737'] |
| neos-1122047 | MR | 0 | 0 | 0 |  |  |

| ヒューリスティクス | 回数 | 秒の和 | 改善した回 | 5 秒以上で改善なしの回 | その秒 | LP 反復 |
|---|---|---|---|---|---|---|
| par:root reduced cost | 26 | 563.1 | 7 | 9 | 302.7 | 762028 |
| RENS | 51 | 311.4 | 25 | 2 | 18.9 | 22025 |
| par:RENS | 25 | 273.1 | 2 | 7 | 260.7 | 212586 |
| par:LP face | 21 | 216.8 | 7 | 2 | 14.1 | 105501 |
| par:RENS (LP before cuts) | 6 | 198.9 | 0 | 3 | 196.9 | 2101 |
| RINS | 43 | 162.1 | 11 | 4 | 60.6 | 0 |
| interior rounding | 46 | 148.2 | 3 | 8 | 119.2 | 11230 |
| feasibility pump | 5 | 80.9 | 0 | 2 | 76.9 | 190769 |
| par:locks | 40 | 55.9 | 7 | 1 | 12.5 | 127983 |
| fractional diving | 5 | 52.2 | 0 | 1 | 48.2 | 85341 |
| vector length diving | 5 | 36.6 | 0 | 1 | 31.3 | 81561 |
| coefficient diving | 5 | 29.1 | 0 | 1 | 24.9 | 74808 |
| clique | 38 | 24.9 | 0 | 1 | 23.7 | 0 |
| vbounds (loose) | 38 | 11.9 | 0 | 1 | 11.9 | 0 |
| local search | 3 | 11.4 | 0 | 2 | 10.0 | 0 |
| randomized rounding | 56 | 6.6 | 9 | 0 | 0.0 | 16753 |
| locks | 40 | 5.8 | 0 | 0 | 0.0 | 0 |
| int shifting | 47 | 4.6 | 5 | 0 | 0.0 | 0 |
| par:vbounds (loose) | 9 | 1.9 | 0 | 0 | 0.0 | 14684 |
| par:clique | 19 | 1.5 | 2 | 0 | 0.0 | 3310 |
| vbounds (tight) | 38 | 1.1 | 0 | 0 | 0.0 | 0 |
| shifting | 49 | 1.1 | 5 | 0 | 0.0 | 0 |
| root reduced cost | 49 | 0.9 | 0 | 0 | 0.0 | 0 |
| par:vbounds (tight) | 9 | 0.7 | 1 | 0 | 0.0 | 2896 |
| RENS (LP before cuts) | 6 | 0.5 | 0 | 0 | 0.0 | 0 |
| trivial | 9 | 0.2 | 2 | 0 | 0.0 | 332 |
| LP face | 49 | 0.1 | 0 | 0 | 0.0 | 0 |
| shift-and-propagate | 5 | 0.1 | 1 | 0 | 0.0 | 886 |
| rounding | 46 | 0.0 | 0 | 0 | 0.0 | 0 |
| ZI round | 57 | 0.0 | 1 | 0 | 0.0 | 0 |
| repair | 3 | 0.0 | 0 | 0 | 0.0 | 0 |
| simple rounding | 57 | 0.0 | 0 | 0 | 0.0 | 0 |
