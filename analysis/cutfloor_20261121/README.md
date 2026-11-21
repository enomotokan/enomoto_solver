# cutfloor: finish_cut の直しの既定の道での影響 (2026-11-21)

## 1. 目的

glc の誤答 (analysis/rootgap_20261121/wrong_widden、README 8 節) の直しは、すべてのカットの共通の仕上げ `finish_cut` を変える。
既定 (環境変数なし) の道で速さ・解けた数・誤答が変わらないかを、P の広い計測で確かめる。

- 小さい係数の吸収の閾値の下限 (`ENOMOTO_MIP_CUT_SMALL_FLOOR`、6a6cc51ba): 閾値を 1e-4 × max(最大係数, 1e-3) にする (HiGHS の `postprocessCut` と同じ)
- 打ち消しの確かめ (`ENOMOTO_MIP_FINISH_CANCEL`、63e236dba): 列ごとに打ち消す前の大きさの 1e-9 倍以下を打ち消しの残りとみなし、1 の吸収で消えない残りが
  1 つでもあるときだけ残りを 0 にする (analysis/rootgap_20261121/README.md 8.2)

## 2. 設定・分担

- `cfg.py`: `def` = 直しなし (`ENOMOTO_MIP_CUT_SMALL_FLOOR=0`・`ENOMOTO_MIP_FINISH_CANCEL=0`、以前の動き)、`fix` = 直しあり (環境変数なし)。どれも 4 スレッド。
- `python plan.py P 6` → shards_P.json (MIPLIB の 58 問 × 種 1〜3 × {def, fix}、60 秒、6 分担、見積り各 0.47 時間)。M は `python plan.py M 20` で作れる (まだ作っていない)。
- 回し方: リポジトリの根で両方の直しが入ったビルド (b0fd1173a 以降) を `pip install -e .` で入れてから、`cd analysis/cutfloor_20261121 && python run.py P K` (K = 0〜5)。res_P_sK.jsonl に足す (途中から再開できる)。
  回したビルドのコミットを下に書き足すこと。
- 集計: `python ci.py 60 def fix res_P_s*.jsonl` (ずらし 10 秒の幾何平均の比の 95% 区間、解けた数、全種で 1.25 倍超の遅れ)。

### 回したビルド

- P 0: b0fd1173a (6a6cc51ba + 63e236dba を含む、pip install -e .、0.1.10)。58 行・crash 0。
- P 3: 1c6643db2 (b0fd1173a 以降、pip install -e .、0.1.10)。58 行・crash 0。
- P 1: 1c6643db2 (b0fd1173a 以降、pip install -e .、0.1.10)。58 行・crash 0。
- P 4: 1c6643db2 (b0fd1173a 以降、pip install -e .、0.1.10)。58 行・crash 0。
- P 5: 1c6643db2 (b0fd1173a 以降、pip install -e .、0.1.10)。58 行・crash 0。
- P 2: 1c6643db2 (b0fd1173a 以降、pip install -e .、0.1.10)。58 行・crash 0。

## 3. 結果 (P、6 分担そろい)

- `python ci.py 60 def fix res_P_s*.jsonl` (ci_P.txt): fix/def のずらし幾何平均の比 0.998 (95% [0.988, 1.007])、解けた数 def 138 / fix 138。
- 全種で 1.25 倍超の遅れ: なし。片方だけ解けた組: なし。optimal 同士の目的値の食い違い: なし。
- 1 回だけの 1.25 倍超は sp150x300d s1 (1.8→3.1 秒、数秒の問題) のみ。時間切れの組の暫定解・下界は、fix が良い組と悪い組が混じり、偏りはない。
- 採用基準 (区間の上端 ≲ +5%、解けた数が減らない、全種 1.25 倍超なし、誤答 0) をすべて満たす。直しは既定のまま (`=0` で戻せる) にしてよい。
