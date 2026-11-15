| 問題 | 組 | 根のヒューリスティクスの始まり (秒) | 終わり | 主のスレッドの秒の和 | 1 秒以上使って改善なし (主) | 根の暫定解の出どころ (最後の 3 つ) | 結果 |
|---|---|---|---|---|---|---|---|
| cvs16r128-89 | P1 | 3.8 | 27.6 | 23.8 | RENS=23.4 | Local-MIP@1.8 Local-MIP@2.2 Local-MIP@3.7 | ['time_limit', '-88.0'] |
| supportcase26 | P1 | 0.4 | 9.8 | 17.9 |  | tree@0.0 tree@0.0 tree@0.0 | ['time_limit', '1802.2337756071754'] |
| blp-ar98 | P1 | 13.4 | 29.3 | 12.3 | interior rounding=1.3 RINS=1.2 | Local-MIP@3.5 RENS@24.4 | ['time_limit', '6433.7893280000035'] |
| ran14x18-disj-8 | P1 | 3.1 | 14.1 | 10.3 | RINS=5.1 | interior rounding@3.2 RENS@8.3 LP face@8.6 | ['time_limit', '3736.00000002697'] |
| neos-1456979 | P1 | 0.7 | 16.9 | 9.5 | feasibility pump=3.3 local search=3.0 interior rounding=1.1 |  | ['time_limit', '321.0'] |
| hypothyroid-k1 | P1 | 7.3 | 11.6 | 4.3 | interior rounding=2.7 | Local-MIP@5.9 find thread@10.3 find thread@11.0 | ['optimal', '-2851.0'] |
| h80x6320d | P1 | 4.0 | 6.3 | 3.1 |  | tree@0.0 RINS@4.0 tree@0.0 | ['optimal', '6382.09905'] |
| neos-860300 | P1 | 5.2 | 7.9 | 2.9 |  | LP face@7.6 tree@0.0 tree@0.0 | ['optimal', '3201.0'] |
| mzzv42z | P1 | 9.0 | 23.5 | 2.8 | interior rounding=2.0 | locks@11.7 RENS@13.4 tree@23.8 | ['time_limit', '-20530.0'] |
| swath1 | P1 | 2.3 | 8.3 | 2.8 | RENS=2.2 | RENS@8.2 find thread@8.2 tree@0.0 | ['optimal', '379.07129575000005'] |
| physiciansched6-2 | P1 | 11.4 | 13.7 | 2.3 | interior rounding=1.7 | Local-MIP@5.9 Local-MIP@10.2 RENS@13.7 | ['optimal', '49323.99999999999'] |
| 10teams | P1 | 1.1 | 3.3 | 2.2 | RENS=2.2 | Local-MIP@0.6 Local-MIP@0.8 | ['optimal', '924.0'] |
| eil33-2 | P1 | 2.8 | 6.8 | 2.0 | RENS=1.9 | Local-MIP@2.5 Local-MIP@2.8 clique@4.9 | ['optimal', '934.007916'] |
| exp-1-500-5-5 | P1 | 2.6 | 5.7 | 2.0 | RINS=1.0 | Local-MIP@0.2 randomized rounding@2.7 | ['optimal', '65887.0'] |
| mc11 | P1 | 3.4 | 5.1 | 1.6 |  | tree@0.1 RENS@4.4 LP face@4.5 | ['optimal', '11689.0'] |
| neos17 | P1 | 0.8 | 2.5 | 1.6 |  | tree@0.0 RENS@2.4 | ['optimal', '0.1500025774225775'] |
| roll3000 | P1 | 1.5 | 5.6 | 1.4 |  | RENS@2.2 RINS@5.6 tree@7.0 | ['optimal', '12890.00000000001'] |
| assign1-5-8 | P1 | 0.9 | 2.0 | 1.1 |  | Local-MIP@0.3 RENS@2.0 | ['time_limit', '212.0'] |
| timtab1 | P1 | 1.4 | 2.0 | 1.0 |  | tree@0.0 RINS@0.2 tree@0.0 | ['time_limit', '772420.9999997803'] |
| rout | P1 | 0.6 | 1.5 | 0.9 |  | Local-MIP@0.3 Local-MIP@0.6 RINS@1.5 | ['optimal', '1077.56'] |
| dcmulti | P1 | 0.3 | 1.1 | 0.8 |  | Local-MIP@0.1 RENS@1.1 | ['optimal', '188182.00000000003'] |
| binkar10_1 | P1 | 0.2 | 1.5 | 0.7 |  | RENS@0.6 root reduced cost@1.2 RINS@1.5 | ['optimal', '6742.200024'] |
| bppc4-08 | P1 | 0.9 | 1.6 | 0.6 |  | Local-MIP@0.3 Local-MIP@0.5 Local-MIP@0.7 | ['time_limit', '54.0'] |
| graph20-20-1rand | P1 | 1.8 | 2.5 | 0.6 |  | Local-MIP@0.3 Local-MIP@1.2 | ['time_limit', '-9.0'] |
| khb05250 | P1 | 0.3 | 1.1 | 0.6 |  | tree@0.0 simple rounding@0.3 randomized rounding@0.3 | ['optimal', '106940226.0'] |
| neos-3083819-nubu | P1 | 0.5 | 1.1 | 0.6 |  | tree@0.0 RINS@0.7 tree@0.0 | ['optimal', '6307996.0'] |
| neos-911970 | P1 | 0.6 | 1.2 | 0.6 |  | interior rounding@0.6 RENS@0.9 RINS@1.2 | ['optimal', '54.760000000000005'] |
| nu25-pr12 | P1 | 0.5 | 1.0 | 0.6 |  | LP face@0.9 interior rounding@0.1 LP face@0.3 | ['optimal', '53905.0'] |
| p200x1188c | P1 | 0.5 | 0.7 | 0.6 |  | tree@0.0 RINS@0.3 tree@0.0 | ['optimal', '15078.0'] |
| 30n20b8 | P1 | 8.7 | 33.0 | 0.4 |  | Local-MIP@3.6 Local-MIP@7.9 | ['time_limit', '302.0'] |
| qnet1 | P1 | 0.6 | 1.0 | 0.4 |  | Local-MIP@0.2 RENS@0.8 tree@0.0 | ['optimal', '16029.692681'] |
| mas74 | P1 | 0.2 | 0.5 | 0.3 |  | tree@0.0 tree@0.0 tree@0.0 | ['time_limit', '11801.185729'] |
| neos5 | P1 | 0.2 | 0.5 | 0.3 |  | tree@0.0 interior rounding@0.2 RENS@0.5 | ['optimal', '15.0'] |
| pk1 | P1 | 0.1 | 0.6 | 0.3 |  | Local-MIP@0.0 RENS@0.3 RINS@0.6 | ['optimal', '10.999999999999996'] |
| qiu | P1 | 1.4 | 2.6 | 0.3 |  | Local-MIP@1.2 RENS@2.6 find thread@2.7 | ['optimal', '-132.87313694699995'] |
| drayage-25-23 | P1 | 0.2 | 0.7 | 0.2 |  | Local-MIP@0.2 RENS@0.5 randomized rounding@0.2 | ['optimal', '101282.64701843262'] |
| mas76 | P1 | 0.2 | 0.4 | 0.2 |  | RENS@0.4 root reduced cost@0.4 tree@0.0 | ['optimal', '40005.05414200001'] |
| neos-960392 | P1 | 0.6 | 0.7 | 0.2 |  | Local-MIP@0.3 Local-MIP@0.6 RENS@0.7 | ['optimal', '-238.0'] |
| blend2 | P1 | 0.0 | 0.2 | 0.1 |  | interior rounding@0.0 root reduced cost@0.2 | ['optimal', '7.598985'] |
| fiber | P1 | 0.5 | 0.6 | 0.1 |  | interior rounding@0.6 LP face@0.1 tree@0.0 | ['optimal', '405935.18'] |
| gen-ip002 | P1 | 0.1 | 0.2 | 0.1 |  | Local-MIP@0.0 ZI round@0.1 RENS@0.2 | ['time_limit', '-4783.7333916'] |
| mik-250-20-75-4 | P1 | 0.5 | 0.5 | 0.1 |  | tree@0.0 ZI round@0.2 RENS@0.3 | ['optimal', '-52300.99999999998'] |
| p0201 | P1 | 0.2 | 0.2 | 0.1 |  | Local-MIP@0.0 Local-MIP@0.2 RENS@0.2 | ['optimal', '7615.0'] |
| sp150x300d | P1 | 0.3 | 0.4 | 0.1 |  | tree@0.0 Local-MIP@0.2 | ['optimal', '69.0'] |
| air03 | P1 | 0 | 0 | 0 |  |  |
| beavma | P1 | 0.2 | 0.3 | 0.0 |  | Local-MIP@0.2 randomized rounding@0.3 | ['optimal', '383285.0'] |
| drayage-100-23 | P1 | 0.2 | 0.2 | 0.0 |  | Local-MIP@0.1 Local-MIP@0.2 interior rounding@0.2 | ['optimal', '103333.87407302856'] |
| flugpl | P1 | 0.1 | 0.1 | 0.0 |  | Local-MIP@0.1 vbounds (loose)@0.1 | ['optimal', '1201499.9999999998'] |
| gen-ip054 | P1 | 0.1 | 0.2 | 0.0 |  | Local-MIP@0.1 Local-MIP@0.1 ZI round@0.1 | ['time_limit', '6841.618386388'] |
| gr4x6 | P1 | 0 | 0 | 0 |  |  |
| gt2 | P1 | 0.1 | 0.1 | 0.0 |  | Local-MIP@0.1 ZI round@0.1 tree@0.0 | ['optimal', '21166.0'] |
| markshare_4_0 | P1 | 0.1 | 0.1 | 0.0 |  | tree@0.0 | ['optimal', '1.0000000000000153'] |
| misc07 | P1 | 0.0 | 0.2 | 0.0 |  | Local-MIP@0.0 | ['optimal', '2809.999999999999'] |
| neos-827175 | P1 | 0 | 0 | 0 |  |  |
| neos8 | P1 | 0 | 0 | 0 |  |  |
| noswot | P1 | 0.1 | 0.1 | 0.0 |  | Local-MIP@0.0 Local-MIP@0.1 | ['optimal', '-41.0'] |
| ns1116954 | P1 | 0 | 0 | 0 |  |  |
| nw04 | P1 | 0 | 0 | 0 |  |  |

| ヒューリスティクス | 回数 | 秒の和 | 改善した回 | 5 秒以上で改善なしの回 | その秒 | LP 反復 |
|---|---|---|---|---|---|---|
| par:root reduced cost | 29 | 81.0 | 5 | 5 | 67.5 | 323250 |
| RENS | 58 | 63.1 | 24 | 1 | 23.4 | 28816 |
| par:RENS | 30 | 33.4 | 5 | 2 | 20.8 | 160826 |
| par:LP face | 31 | 28.3 | 8 | 1 | 9.7 | 70910 |
| RINS | 60 | 25.9 | 10 | 1 | 5.1 | 0 |
| interior rounding | 52 | 13.8 | 9 | 0 | 0.0 | 2893 |
| par:RENS (LP before cuts) | 3 | 5.2 | 0 | 1 | 5.2 | 265 |
| par:locks | 39 | 3.4 | 1 | 0 | 0.0 | 11696 |
| feasibility pump | 1 | 3.3 | 0 | 0 | 0.0 | 10520 |
| local search | 1 | 3.0 | 0 | 0 | 0.0 | 0 |
| locks | 41 | 1.3 | 0 | 0 | 0.0 | 0 |
| randomized rounding | 76 | 1.1 | 8 | 0 | 0.0 | 2113 |
| clique | 39 | 0.7 | 0 | 0 | 0.0 | 0 |
| fractional diving | 1 | 0.7 | 0 | 0 | 0.0 | 2209 |
| vector length diving | 1 | 0.6 | 0 | 0 | 0.0 | 2197 |
| coefficient diving | 1 | 0.6 | 0 | 0 | 0.0 | 2204 |
| RENS (LP before cuts) | 3 | 0.4 | 0 | 0 | 0.0 | 0 |
| int shifting | 51 | 0.3 | 0 | 0 | 0.0 | 0 |
| par:clique | 14 | 0.2 | 1 | 0 | 0.0 | 1147 |
| root reduced cost | 52 | 0.1 | 0 | 0 | 0.0 | 0 |
| LP face | 54 | 0.1 | 0 | 0 | 0.0 | 0 |
| vbounds (loose) | 39 | 0.1 | 0 | 0 | 0.0 | 0 |
| par:vbounds (loose) | 7 | 0.0 | 1 | 0 | 0.0 | 1270 |
| shifting | 52 | 0.0 | 1 | 0 | 0.0 | 0 |
| rounding | 51 | 0.0 | 0 | 0 | 0.0 | 0 |
| vbounds (tight) | 39 | 0.0 | 0 | 0 | 0.0 | 0 |
| trivial | 7 | 0.0 | 0 | 0 | 0.0 | 1 |
| par:vbounds (tight) | 7 | 0.0 | 0 | 0 | 0.0 | 9 |
| ZI round | 77 | 0.0 | 4 | 0 | 0.0 | 0 |
| shift-and-propagate | 1 | 0.0 | 0 | 0 | 0.0 | 0 |
| simple rounding | 78 | 0.0 | 2 | 0 | 0.0 | 0 |
| repair | 1 | 0.0 | 0 | 0 | 0.0 | 0 |
