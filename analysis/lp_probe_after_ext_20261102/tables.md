# ENOMOTO_MIP_LP_PROBE_AFTER_EXT の計測

## 1. 対象の問題 (回ごと、4 スレッド・60 秒)

root_probe・restart の欄は NB_DIAG の (LP / 特異で捨てた LP / 反復 / 秒)。固定は `MIP: LP probing: ... fixed N` の N (前処理のかけ直しの中の分も並べる)。

| 問題 | 設定 | 回 | 結果 | 時間 | 前処理の LP (秒 / 反復) | 固定 | 根に渡した基底 | 根の LP (反復 / 秒) | root_probe | restart |
|---|---|---|---|---|---|---|---|---|---|---|
| ns1116954 | base | 0 | optimal 0.0 | 3.6 | – | 0 | – | – | – | – |
| ns1116954 | base | 1 | optimal 0.0 | 3.4 | – | 0 | – | – | – | – |
| ns1116954 | base | 2 | optimal 0.0 | 4.0 | – | 0 | – | – | – | – |
| ns1116954 | ext | 0 | optimal 0.0 | 3.6 | – | 0 | – | – | – | – |
| ns1116954 | ext | 1 | optimal 0.0 | 3.9 | – | 0 | – | – | – | – |
| ns1116954 | ext | 2 | optimal 0.0 | 3.7 | – | 0 | – | – | – | – |
| ns1116954 | base_nofj | 0 | optimal 0.0 | 33.5 | 14.1 / 3225 | 0 | root LP basis from the presolve LP probing: used (columns only) | 3892 / 11.7 | 1 / 1 / 3892 / 11.6 | – |
| ns1116954 | base_nofj | 1 | optimal 0.0 | 31.7 | 11.9 / 3225 | 0 | root LP basis from the presolve LP probing: used (columns only) | 3892 / 12.1 | 1 / 1 / 3892 / 11.9 | – |
| ns1116954 | base_nofj | 2 | optimal 0.0 | 32.2 | 12.4 / 3225 | 0 | root LP basis from the presolve LP probing: used (columns only) | 3892 / 11.6 | 1 / 1 / 3892 / 11.5 | – |
| ns1116954 | ext_nofj | 0 | optimal 0.0 | 35.5 | 14.7 / 4085 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.3 | 1 / 0 / 1 / 0.2 | – |
| ns1116954 | ext_nofj | 1 | optimal 0.0 | 35.1 | 14.2 / 4085 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.3 | 1 / 0 / 1 / 0.2 | – |
| ns1116954 | ext_nofj | 2 | optimal 0.0 | 35.6 | 15.3 / 4085 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.3 | 1 / 0 / 1 / 0.3 | – |
| mzzv42z | base | 0 | time_limit -20470.0 | 60.0 | 3.0 / 8592 | 0 | root LP basis from the presolve LP probing: used (columns only) | 6877 / 1.8 | 1 / 1 / 6877 / 1.8 | 1 / 1 / 23427 / 13.0 |
| mzzv42z | base | 1 | time_limit -20470.0 | 60.0 | 2.6 / 8592 | 0 | root LP basis from the presolve LP probing: used (columns only) | 6877 / 1.9 | 1 / 1 / 6877 / 1.9 | 1 / 1 / 11581 / 4.9 |
| mzzv42z | base | 2 | time_limit -20500.0 | 60.0 | 3.2 / 8592 | 0 | root LP basis from the presolve LP probing: used (columns only) | 6877 / 2.3 | 1 / 1 / 6877 / 2.2 | 1 / 1 / 13178 / 5.1 |
| mzzv42z | ext | 0 | time_limit -20490.0 | 60.0 | 3.5 / 7553 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.0 | 1 / 0 / 1 / 0.0 | 1 / 1 / 17652 / 9.0 |
| mzzv42z | ext | 1 | time_limit -20470.0 | 60.0 | 2.3 / 7553 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.1 | 1 / 0 / 1 / 0.0 | 1 / 1 / 14521 / 5.9 |
| mzzv42z | ext | 2 | time_limit -20470.0 | 60.0 | 2.5 / 7553 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.1 | 1 / 0 / 1 / 0.0 | 1 / 1 / 22003 / 11.3 |
| neos-960392 | base | 0 | optimal -238.0 | 29.2 | 2.7 / 7614 | 0 | root LP basis from the presolve LP probing: used (columns only) | 7447 / 3.0 | 1 / 1 / 7447 / 2.9 | – |
| neos-960392 | base | 1 | optimal -238.0 | 27.9 | 2.6 / 7614 | 0 | root LP basis from the presolve LP probing: used (columns only) | 7447 / 2.7 | 1 / 1 / 7447 / 2.6 | – |
| neos-960392 | base | 2 | optimal -238.0 | 30.1 | 2.7 / 7614 | 0 | root LP basis from the presolve LP probing: used (columns only) | 7447 / 3.1 | 1 / 1 / 7447 / 3.0 | – |
| neos-960392 | ext | 0 | optimal -238.0 | 52.5 | 2.6 / 7184 | 0 | root LP basis from the presolve LP probing: used (with rows) | 2665 / 2.0 | 1 / 0 / 2665 / 1.9 | – |
| neos-960392 | ext | 1 | optimal -238.0 | 49.7 | 2.3 / 7184 | 0 | root LP basis from the presolve LP probing: used (with rows) | 2665 / 1.7 | 1 / 0 / 2665 / 1.7 | – |
| neos-960392 | ext | 2 | optimal -238.0 | 50.3 | 2.4 / 7184 | 0 | root LP basis from the presolve LP probing: used (with rows) | 2665 / 1.9 | 1 / 0 / 2665 / 1.9 | – |
| physiciansched6-2 | base | 0 | time_limit None | 60.0 | 3.6 / 6733 | 0 | root LP basis from the presolve LP probing: used (columns only) | 6074 / 2.7 | 1 / 1 / 6074 / 2.7 | – |
| physiciansched6-2 | base | 1 | optimal 49323.99999999999 | 27.3 | 3.4 / 6528 | 0 | root LP basis from the presolve LP probing: used (columns only) | 6074 / 2.3 | 1 / 1 / 6074 / 2.3 | – |
| physiciansched6-2 | base | 2 | optimal 49323.99999999999 | 25.4 | 3.3 / 6733 | 0 | root LP basis from the presolve LP probing: used (columns only) | 6074 / 2.5 | 1 / 1 / 6074 / 2.4 | – |
| physiciansched6-2 | ext | 0 | time_limit 49324.0 | 60.0 | 2.7 / 6464 | 0 | root LP basis from the presolve LP probing: used (with rows) | 824 / 0.4 | 1 / 0 / 824 / 0.4 | – |
| physiciansched6-2 | ext | 1 | time_limit 49424.0 | 60.0 | 3.0 / 6464 | 0 | root LP basis from the presolve LP probing: used (with rows) | 824 / 0.5 | 1 / 0 / 824 / 0.5 | – |
| physiciansched6-2 | ext | 2 | time_limit 49824.0 | 60.0 | 2.8 / 6464 | 0 | root LP basis from the presolve LP probing: used (with rows) | 824 / 0.4 | 1 / 0 / 824 / 0.4 | – |
| neos-827175 | base | 0 | optimal 112.00152 | 5.7 | 0.6 / 8163 | 0 | root LP basis from the presolve LP probing: used (columns only) | 8287 / 0.7 | 1 / 1 / 8287 / 0.7 | – |
| neos-827175 | base | 1 | optimal 112.00152 | 5.6 | 0.7 / 8163 | 0 | root LP basis from the presolve LP probing: used (columns only) | 8287 / 0.7 | 1 / 1 / 8287 / 0.7 | – |
| neos-827175 | base | 2 | optimal 112.00152 | 5.8 | 0.8 / 8163 | 0 | root LP basis from the presolve LP probing: used (columns only) | 8287 / 0.7 | 1 / 1 / 8287 / 0.7 | – |
| neos-827175 | ext | 0 | optimal 112.00152 | 5.0 | 0.6 / 8270 | 0 | root LP basis from the presolve LP probing: used (with rows) | 748 / 0.2 | 1 / 0 / 748 / 0.2 | – |
| neos-827175 | ext | 1 | optimal 112.00152 | 6.3 | 0.6 / 8270 | 0 | root LP basis from the presolve LP probing: used (with rows) | 748 / 0.3 | 1 / 0 / 748 / 0.3 | – |
| neos-827175 | ext | 2 | optimal 112.00152 | 5.9 | 0.7 / 8270 | 0 | root LP basis from the presolve LP probing: used (with rows) | 748 / 0.2 | 1 / 0 / 748 / 0.2 | – |
| hypothyroid-k1 | base | 0 | optimal -2851.0 | 28.4 | 5.4 / 3067 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.2 | 1 / 0 / 1 / 0.2 | – |
| hypothyroid-k1 | base | 1 | optimal -2851.0 | 29.0 | 6.3 / 3067 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.2 | 1 / 0 / 1 / 0.2 | – |
| hypothyroid-k1 | base | 2 | optimal -2851.0 | 28.6 | 5.4 / 3067 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.2 | 1 / 0 / 1 / 0.2 | – |
| hypothyroid-k1 | ext | 0 | optimal -2851.0 | 28.0 | 5.0 / 3067 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.2 | 1 / 0 / 1 / 0.2 | – |
| hypothyroid-k1 | ext | 1 | optimal -2851.0 | 30.2 | 6.1 / 3067 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.2 | 1 / 0 / 1 / 0.2 | – |
| hypothyroid-k1 | ext | 2 | optimal -2851.0 | 31.5 | 5.5 / 3067 | 0 | root LP basis from the presolve LP probing: used (with rows) | 1 / 0.2 | 1 / 0 / 1 / 0.2 | – |
