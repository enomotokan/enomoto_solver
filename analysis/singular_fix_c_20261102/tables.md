# ENOMOTO_MIP_ROOT_BASIS_FIX の計測

## 1. 根の LP が重い 5 問 (回ごと)

| 問題 | 設定 | 回 | 結果 | 時間 | 根の LP (反復 / 秒) | root_probe (LP / 特異 / 反復 / 秒) | restart (LP / 特異 / 反復 / 秒) | 再スタートの後の根の LP |
|---|---|---|---|---|---|---|---|---|
| ns1116954 | base | 0 | optimal 0.0 | 42.8 | 3892 / 15.9 | 1 / 1 / 3892 / 15.8 | – | – |
| ns1116954 | base | 1 | optimal 0.0 | 39.1 | 3892 / 15.3 | 1 / 1 / 3892 / 15.2 | – | – |
| ns1116954 | base | 2 | optimal 0.0 | 42.5 | 3892 / 16.4 | 1 / 1 / 3892 / 16.4 | – | – |
| ns1116954 | fix | 0 | optimal 0.0 | 45.6 | 3763 / 20.4 | 1 / 0 / 3763 / 19.9 | – | – |
| ns1116954 | fix | 1 | optimal 0.0 | 43.0 | 3763 / 18.9 | 1 / 0 / 3763 / 18.5 | – | – |
| ns1116954 | fix | 2 | optimal 0.0 | 49.9 | 3763 / 21.1 | 1 / 0 / 3763 / 20.8 | – | – |
| mzzv42z | base | 0 | time_limit -20410.0 | 60.0 | 6877 / 2.1 | 1 / 1 / 6877 / 2.1 | 1 / 1 / 11984 / 5.4 | Optimal 11984 / 5.4 |
| mzzv42z | base | 1 | time_limit -20460.0 | 60.0 | 6877 / 2.0 | 1 / 1 / 6877 / 2.0 | 1 / 1 / 17509 / 7.8 | Optimal 17509 / 7.8 |
| mzzv42z | base | 2 | time_limit -20410.0 | 60.0 | 6877 / 2.4 | 1 / 1 / 6877 / 2.4 | 1 / 1 / 10790 / 5.4 | Optimal 10790 / 5.5 |
| mzzv42z | fix | 0 | time_limit -20430.0 | 60.0 | 1652 / 1.0 | 1 / 0 / 1652 / 0.9 | – | – |
| mzzv42z | fix | 1 | time_limit -20540.0 | 60.0 | 1652 / 1.0 | 1 / 0 / 1652 / 0.9 | – | – |
| mzzv42z | fix | 2 | time_limit -20460.0 | 60.0 | 1652 / 1.1 | 1 / 0 / 1652 / 1.0 | – | – |
| neos-960392 | base | 0 | optimal -238.0 | 30.7 | 7447 / 3.0 | 1 / 1 / 7447 / 2.9 | – | – |
| neos-960392 | base | 1 | optimal -238.0 | 30.8 | 7447 / 2.6 | 1 / 1 / 7447 / 2.6 | – | – |
| neos-960392 | base | 2 | optimal -238.0 | 35.5 | 7447 / 3.1 | 1 / 1 / 7447 / 3.1 | – | – |
| neos-960392 | fix | 0 | time_limit -235.0 | 60.0 | 3703 / 2.6 | 1 / 0 / 3703 / 2.6 | – | – |
| neos-960392 | fix | 1 | time_limit -235.0 | 60.0 | 3703 / 2.9 | 1 / 0 / 3703 / 2.8 | – | – |
| neos-960392 | fix | 2 | time_limit -235.0 | 60.0 | 3703 / 2.9 | 1 / 0 / 3703 / 2.8 | – | – |
| physiciansched6-2 | base | 0 | optimal 49323.99999999999 | 36.4 | 6074 / 2.7 | 1 / 1 / 6074 / 2.7 | – | – |
| physiciansched6-2 | base | 1 | time_limit 49324.00000000001 | 60.0 | 6074 / 2.9 | 1 / 1 / 6074 / 2.9 | – | – |
| physiciansched6-2 | base | 2 | optimal 49323.99999999999 | 39.6 | 6074 / 2.8 | 1 / 1 / 6074 / 2.8 | – | – |
| physiciansched6-2 | fix | 0 | time_limit 49324.0 | 60.0 | 3295 / 2.0 | 1 / 0 / 3295 / 1.9 | – | – |
| physiciansched6-2 | fix | 1 | time_limit 49624.0 | 60.0 | 5672 / 3.2 | 1 / 0 / 5672 / 3.1 | – | – |
| physiciansched6-2 | fix | 2 | time_limit 49524.0 | 60.0 | 5672 / 3.5 | 1 / 0 / 5672 / 3.5 | – | – |
| neos-827175 | base | 0 | optimal 112.00152 | 5.9 | 8287 / 0.7 | 1 / 1 / 8287 / 0.7 | – | – |
| neos-827175 | base | 1 | optimal 112.00152 | 6.7 | 8287 / 0.8 | 1 / 1 / 8287 / 0.8 | – | – |
| neos-827175 | base | 2 | optimal 112.00152 | 6.4 | 8287 / 0.9 | 1 / 1 / 8287 / 0.8 | – | – |
| neos-827175 | fix | 0 | optimal 112.00152 | 7.1 | 451 / 0.2 | 1 / 0 / 451 / 0.2 | – | – |
| neos-827175 | fix | 1 | optimal 112.00152 | 7.8 | 451 / 0.2 | 1 / 0 / 451 / 0.2 | – | – |
| neos-827175 | fix | 2 | optimal 112.00152 | 7.5 | 451 / 0.2 | 1 / 0 / 451 / 0.2 | – | – |

## 2. 58 問 (回 2) の root_probe・restart の合計

| 設定 | 場所 | 問題 | LP | 特異で捨てた LP | 反復 | 時間 |
|---|---|---|---|---|---|---|
| base | root_probe | 13 | 13 | 9 | 37456 | 26.3 秒 |
| base | restart | 23 | 36 | 32 | 29582 | 6.6 秒 |
| base | slack_retry | 10 | 49 | 0 | 44111 | 8.2 秒 |
| fix | root_probe | 13 | 13 | 0 | 18370 | 28.9 秒 |
| fix | restart | 14 | 21 | 0 | 11203 | 1.3 秒 |
| fix | slack_retry | 11 | 55 | 0 | 60345 | 12.9 秒 |

## 3. 58 問 (回 2) の全体 (4 スレッド・60 秒、SGM はずらし 10 秒、解けなかった問題は 60 秒)

| 設定 | 問題 | 解けた | SGM |
|---|---|---|---|
| base | 58 | 42 | 19.26 秒 |
| fix | 58 | 41 | 20.74 秒 |

比 (fix / base): +5.1% (ずらした幾何平均)

### 問題ごと (root_probe か restart のある問題と、時間が 10% 以上かつ 1 秒以上違う問題)

| 問題 | base 時間 | fix 時間 | base 根の LP | fix 根の LP | base root_probe/restart | fix root_probe/restart | fix の基底 |
|---|---|---|---|---|---|---|---|
| drayage-100-23 | 1.8 | 4.1 | 574 / 0.0 | 574 / 0.0 | – | – | – |
| drayage-25-23 | 9.1 | 7.9 | 625 / 0.0 | 625 / 0.1 | resta 3/3 特異 2060 反復 0.1 秒 | resta 0/2 特異 978 反復 0.1 秒 | matched: 717, tight: 3, loose: 20, dropped: 1, trimmed: 1, padded: 0, swapped: 3,; matched: 520, tight: 10, loose: 15, dropped: 10, trimmed: 0, padded: 1, swapped: 4, |
| exp-1-500-5-5 | 7.7 | 5.6 | 611 / 0.0 | 611 / 0.0 | resta 4/4 特異 4714 反復 0.1 秒 | resta 0/1 特異 1148 反復 0.2 秒 | matched: 880, tight: 0, loose: 0, dropped: 23, trimmed: 0, padded: 0, swapped: 0,; matched: 822, tight: 2, loose: 0, dropped: 43, trimmed: 0, padded: 3, swapped: 4,; matched: 836, tight: 0, loose: 0, dropped: 23, trimmed: 0, padded: 1, swapped: 1,; matched: 851, tight: 0, loose: 0, dropped: 9, trimmed: 0, padded: 1, swapped: 0, |
| h80x6320d | 50.6 | 54.2 | 257 / 0.1 | 257 / 0.1 | resta 2/2 特異 1910 反復 0.2 秒 | resta 0/2 特異 2250 反復 0.5 秒 | matched: 1546, tight: 5, loose: 0, dropped: 4, trimmed: 2, padded: 0, swapped: 1,; matched: 1431, tight: 11, loose: 2, dropped: 3, trimmed: 0, padded: 5, swapped: 1, |
| hypothyroid-k1 | 27.9 | 29.0 | 1 / 0.3 | 1 / 0.3 | root_ 0/1 特異 1 反復 0.2 秒 | root_ 0/1 特異 1 反復 0.2 秒 | – |
| mzzv42z | 60.0 | 60.0 | 6877 / 2.4 | 1652 / 1.1 | root_ 1/1 特異 6877 反復 2.4 秒; resta 1/1 特異 10790 反復 5.4 秒 | root_ 0/1 特異 1652 反復 1.0 秒 | matched: 9190, tight: 231, loose: 900, dropped: 0, trimmed: 0, padded: 218, swapped: 9,; matched: 7856, tight: 523, loose: 1525, dropped: 396, trimmed: 0, padded: 253, swapped: 114,; matched: 6755, tight: 430, loose: 982, dropped: 261, trimmed: 0, padded: 202, swapped: 135, |
| neos-3083819-nubu | 26.6 | 17.2 | 1 / 0.0 | 1 / 0.0 | root_ 0/1 特異 1 反復 0.0 秒; resta 2/2 特異 958 反復 0.0 秒 | root_ 0/1 特異 1 反復 0.0 秒 | matched: 370, tight: 46, loose: 4, dropped: 55, trimmed: 5, padded: 0, swapped: 5,; matched: 280, tight: 27, loose: 6, dropped: 66, trimmed: 4, padded: 0, swapped: 4, |
| neos-827175 | 6.4 | 7.5 | 8287 / 0.9 | 451 / 0.2 | root_ 1/1 特異 8287 反復 0.8 秒 | root_ 0/1 特異 451 反復 0.2 秒 | matched: 13917, tight: 11, loose: 111, dropped: 0, trimmed: 0, padded: 10, swapped: 1, |
| neos-860300 | 32.8 | 31.4 | 320 / 0.1 | 65 / 0.1 | root_ 1/1 特異 320 反復 0.0 秒; resta 1/1 特異 408 反復 0.1 秒 | root_ 0/1 特異 65 反復 0.0 秒; resta 0/1 特異 346 反復 0.1 秒 | matched: 594, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0,; matched: 623, tight: 2, loose: 0, dropped: 1, trimmed: 0, padded: 0, swapped: 0, |
| neos-911970 | 19.5 | 26.2 | 200 / 0.0 | 200 / 0.0 | resta 1/1 特異 1146 反復 0.0 秒 | resta 0/1 特異 132 反復 0.0 秒 | matched: 216, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0, |
| neos-960392 | 35.5 | 60.0 | 7447 / 3.1 | 3703 / 2.9 | root_ 1/1 特異 7447 反復 3.1 秒 | root_ 0/1 特異 3703 反復 2.8 秒 | matched: 3268, tight: 802, loose: 22, dropped: 0, trimmed: 0, padded: 678, swapped: 63, |
| neos17 | 10.3 | 10.7 | 471 / 0.0 | 471 / 0.0 | resta 1/1 特異 396 反復 0.0 秒 | resta 0/2 特異 2 反復 0.0 秒 | matched: 659, tight: 2, loose: 0, dropped: 8, trimmed: 0, padded: 0, swapped: 0,; matched: 668, tight: 0, loose: 2, dropped: 10, trimmed: 0, padded: 0, swapped: 0, |
| ns1116954 | 42.5 | 49.9 | 3892 / 16.4 | 3763 / 21.1 | root_ 1/1 特異 3892 反復 16.4 秒 | root_ 0/1 特異 3763 反復 20.8 秒 | matched: 124983, tight: 1117, loose: 1559, dropped: 0, trimmed: 0, padded: 990, swapped: 111, |
| nu25-pr12 | 4.5 | 5.0 | 1487 / 0.0 | 1487 / 0.1 | resta 1/1 特異 905 反復 0.0 秒 | – | matched: 1428, tight: 81, loose: 11, dropped: 165, trimmed: 0, padded: 23, swapped: 26, |
| p200x1188c | 4.7 | 4.3 | 559 / 0.0 | 559 / 0.0 | resta 2/2 特異 634 反復 0.0 秒 | – | matched: 273, tight: 45, loose: 10, dropped: 603, trimmed: 0, padded: 18, swapped: 11,; matched: 163, tight: 27, loose: 3, dropped: 53, trimmed: 14, padded: 0, swapped: 10,; matched: 145, tight: 20, loose: 10, dropped: 19, trimmed: 3, padded: 0, swapped: 1, |
| physiciansched6-2 | 39.6 | 60.0 | 6074 / 2.8 | 5672 / 3.5 | root_ 1/1 特異 6074 反復 2.8 秒 | root_ 0/1 特異 5672 反復 3.5 秒 | matched: 4910, tight: 357, loose: 3154, dropped: 710, trimmed: 0, padded: 31, swapped: 220, |
| sp150x300d | 0.9 | 0.8 | 231 / 0.0 | 231 / 0.0 | resta 2/2 特異 583 反復 0.0 秒 | – | matched: 247, tight: 52, loose: 10, dropped: 51, trimmed: 19, padded: 0, swapped: 20,; matched: 273, tight: 23, loose: 6, dropped: 18, trimmed: 0, padded: 4, swapped: 11, |
| swath1 | 29.5 | 44.3 | 116 / 0.0 | 116 / 0.0 | resta 2/2 特異 290 反復 0.0 秒 | resta 0/1 特異 1 反復 0.0 秒 | matched: 502, tight: 3, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0, |
| pk1 | 29.4 | 22.8 | 0 / 0.0 | 34 / 0.0 | – | – | – |
| mas74 | 60.0 | 60.0 | 70 / 0.0 | 70 / 0.0 | resta 0/1 特異 73 反復 0.0 秒 | resta 0/1 特異 1 反復 0.0 秒 | matched: 25, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0, |
| mas76 | 12.8 | 14.8 | 74 / 0.0 | 74 / 0.0 | resta 0/1 特異 89 反復 0.0 秒 | resta 0/1 特異 1 反復 0.0 秒 | matched: 24, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0, |
| flugpl | 0.1 | 0.1 | 1 / 0.0 | 1 / 0.0 | root_ 0/1 特異 1 反復 0.0 秒 | root_ 0/1 特異 1 反復 0.0 秒 | – |
| dcmulti | 2.6 | 4.5 | 353 / 0.0 | 1 / 0.0 | root_ 1/1 特異 353 反復 0.0 秒 | root_ 0/1 特異 1 反復 0.0 秒; resta 0/1 特異 1 反復 0.0 秒 | matched: 224, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0,; matched: 335, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0, |
| gt2 | 0.1 | 0.1 | 70 / 0.0 | 70 / 0.0 | resta 0/1 特異 82 反復 0.0 秒 | – | matched: 42, tight: 0, loose: 0, dropped: 2, trimmed: 0, padded: 1, swapped: 1, |
| khb05250 | 0.9 | 0.9 | 114 / 0.0 | 114 / 0.0 | resta 1/1 特異 269 反復 0.0 秒 | resta 0/2 特異 666 反復 0.0 秒 | matched: 216, tight: 0, loose: 0, dropped: 0, trimmed: 17, padded: 0, swapped: 10,; matched: 219, tight: 0, loose: 0, dropped: 0, trimmed: 3, padded: 0, swapped: 3, |
| qnet1 | 3.0 | 8.8 | 376 / 0.0 | 376 / 0.0 | resta 1/1 特異 591 反復 0.0 秒 | resta 0/3 特異 5674 反復 0.4 秒 | matched: 371, tight: 25, loose: 0, dropped: 15, trimmed: 0, padded: 2, swapped: 2,; matched: 378, tight: 19, loose: 7, dropped: 4, trimmed: 0, padded: 1, swapped: 1,; matched: 396, tight: 14, loose: 12, dropped: 5, trimmed: 0, padded: 3, swapped: 2,; matched: 396, tight: 13, loose: 13, dropped: 6, trimmed: 2, padded: 0, swapped: 2, |
| 10teams | 6.0 | 34.0 | 1200 / 0.1 | 1199 / 0.1 | – | – | – |
| blend2 | 0.6 | 0.4 | 286 / 0.0 | 12 / 0.0 | root_ 1/1 特異 286 反復 0.0 秒; resta 1/1 特異 172 反復 0.0 秒 | root_ 0/1 特異 12 反復 0.0 秒 | matched: 168, tight: 0, loose: 1, dropped: 0, trimmed: 1, padded: 0, swapped: 0,; matched: 93, tight: 0, loose: 0, dropped: 58, trimmed: 0, padded: 0, swapped: 0, |
| fiber | 0.9 | 1.8 | 187 / 0.0 | 187 / 0.0 | resta 2/2 特異 475 反復 0.0 秒 | – | matched: 274, tight: 8, loose: 4, dropped: 43, trimmed: 0, padded: 7, swapped: 6,; matched: 177, tight: 20, loose: 11, dropped: 44, trimmed: 0, padded: 0, swapped: 9, |
| 30n20b8 | 60.0 | 60.0 | 3915 / 0.5 | 3046 / 0.4 | root_ 1/1 特異 3915 反復 0.5 秒 | root_ 0/1 特異 3046 反復 0.4 秒 | matched: 491, tight: 0, loose: 0, dropped: 0, trimmed: 0, padded: 0, swapped: 0, |
| binkar10_1 | 8.6 | 10.7 | 539 / 0.0 | 539 / 0.0 | resta 2/2 特異 1184 反復 0.0 秒 | resta 0/2 特異 2 反復 0.0 秒 | matched: 799, tight: 0, loose: 0, dropped: 13, trimmed: 0, padded: 0, swapped: 0,; matched: 778, tight: 0, loose: 0, dropped: 12, trimmed: 0, padded: 0, swapped: 0,; matched: 750, tight: 0, loose: 0, dropped: 17, trimmed: 0, padded: 0, swapped: 0, |
| nw04 | 13.8 | 15.6 | 115 / 0.4 | 115 / 0.4 | – | – | – |
| beavma | 0.6 | 0.5 | 179 / 0.0 | 179 / 0.0 | resta 2/2 特異 431 反復 0.0 秒 | – | matched: 341, tight: 19, loose: 11, dropped: 54, trimmed: 0, padded: 36, swapped: 32,; matched: 93, tight: 78, loose: 17, dropped: 77, trimmed: 0, padded: 14, swapped: 7, |
| blp-ar98 | 60.0 | 60.0 | 2 / 0.1 | 2 / 0.0 | root_ 0/1 特異 2 反復 0.0 秒; resta 1/1 特異 1280 反復 0.4 秒 | root_ 0/1 特異 2 反復 0.0 秒; resta 0/1 特異 1 反復 0.0 秒 | matched: 1364, tight: 0, loose: 0, dropped: 2, trimmed: 0, padded: 0, swapped: 0, |
| mik-250-20-75-4 | 7.0 | 7.6 | 83 / 0.0 | 83 / 0.0 | resta 0/1 特異 142 反復 0.0 秒 | – | matched: 59, tight: 19, loose: 51, dropped: 0, trimmed: 19, padded: 0, swapped: 13, |
