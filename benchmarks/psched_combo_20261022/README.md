# physiciansched6-2 (最適値 49324)、4 スレッド・60 秒、各設定 10 回 (交互に実行)

HEAD 2dceb52。全 40 回で返った解は元の MPS で実行可能、目的値の再計算と一致。`MIP: WARNING` なし。

| 設定 | 解けた回数 | 暫定解 (回数) | 下界 (回数) | ノード数 | 最初の暫定解 (時刻・値) |
|---|---|---|---|---|---|
| newdef | 0/10 | 51029 (10) | 48587.4 (5), 48688.3 (5) | 3–162 | 9.0–9.5 秒, 51029 |
| e2hl | 0/10 | 49324 (4), 50626 (6) | 49065.3 (2), 49153.2 (1), 49157.3 (3), 49265.7 (1), 49299 (3) | 0–2 | 7.5–8.0 秒, 50626 |
| e2hsel | 0/10 | 50626 (10) | 48843.7 (7), 49290.7 (3) | 1–28 | 7.9–8.4 秒, 50626 |
| combo | 0/10 | 49424 (1), 49524 (1), 50626 (8) | 48843.7 (2), 48982.2 (3), 49148.8 (2), 49164.5 (1), 49324 (2) | 0–3 | 7.8–8.3 秒, 50626 |

全回が時間切れ (time_limit) のため、解けた回の平均時間はない。

# 55 問での確認 (combo の環境変数あり/なし、4 スレッド・10 秒、ENOMOTO_MIP_LOG=1)

- 主問題の前処理 (ログの最初の `MIP: presolve: rows` 行) は 55 問すべてで両設定一致。2 行目以降はサブ MIP の前処理で、newdef 同士でも回ごとに変わる (check_newdef_repeat.jsonl)。
- 全 110 回で解は実行可能・目的値一致、`MIP: WARNING` なし、異常終了なし。

| 問題 | 主問題で代入 (EQUIV2) | サブ MIP で代入 | ROOT_SUBMIP_HL (RINS (HL) 行数) | newdef 状態・目的値 | combo 状態・目的値 |
|---|---|---|---|---|---|
| 10teams |  |  | 1 | time_limit 976.0 | time_limit 976.0 |
| app1-1 |  |  | 1 | time_limit 0.0 | time_limit 0.0 |
| beavma |  | ○ |  | optimal 383285.0 | optimal 383285.0 |
| fiber |  | ○ |  | optimal 405935.18 | optimal 405935.18 |
| graph20-20-1rand | ○ |  | 1 | time_limit -8.0 | time_limit -8.0 |
| misc07 |  |  | 1 | time_limit 2809.999999999999 | time_limit 2809.9999999999986 |
| mzzv42z |  |  | 1 | time_limit -9108.0 | time_limit -9108.0 |
| neos-950242 |  |  | 1 | time_limit 20.0 | time_limit 20.0 |
| nw04 |  |  | 1 | time_limit 26126.0 | time_limit 22494.0 |
| physiciansched6-2 | ○ |  |  | time_limit None | time_limit None |
| supportcase26 |  |  | 1 | time_limit 1776.7565080993377 | time_limit 1916.8814793901856 |
| timtab1 | ○ |  |  | time_limit 832773.9999998402 | time_limit 804770.99999985 |
