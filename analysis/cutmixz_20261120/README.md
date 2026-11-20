# CUTMIX=z の広い計測 (P2-INF、2026-11-20)

親の依頼 (利用者の了承): `ENOMOTO_MIP_CUTMIX=z` を広く測る。z は「全部整数で係数が絶対値 2 以下の行の割合 (intsmall、根のカットの
ループの初め・前処理の後の問題で数える) が 0.5 以上の問題でだけ zero-half を使う」(src/mip/cutmix.rs の `zh_on`)。既定では zero-half は
切ってある (`ENOMOTO_MIP_NO_ZEROHALF` の既定 `1`)。きっかけは seymour (intsmall 1.0): 4 スレッド 600 秒で下界 413.0 → 416.0 (SCIP 416.69)
(analysis/vsscip_slow_20261118/tree3/README.md の 3 節)。zero-half を全部の問題で入れると 58 問で SGM +1.1% (analysis/cutmix_20261111.md)。

## 1. z が働く問題 (zscan.py → zscan.jsonl)

M (analysis/vsscip_20261120 の Mittelmann 186 問) を 1 スレッド・種 1・`CUTMIX=z`・`ENOMOTO_MIP_LOG=1` で解き始め、最初の
`CUTMIX types` の行の intsmall を取って止めた (90 秒まで、3c0625a82 のビルド)。

- intsmall ≥ 0.5: **68 問**。90 秒で根のカットに届かず不明: 12 問 (chromaticindex1024-7・chromaticindex512-7・cod105・
  cryptanalysiskb128n5obj14・cryptanalysiskb128n5obj16・enlight_hard・neos-3004026-krka・neos-3988577-wolgan・neos-787933・neos-873061・
  opm2-z10-s4・rail01)。不明は計測に入れる (計 80 問)。残りの 106 問は z が働かず def と同じ道筋なので回さない。
- 割合は再スタートのたびに数え直すので、根で 0.5 未満でも再スタートの後に働く問題はありうる (この調べでは拾わない)。

## 2. 分担 (plan.py)

| 集合 | 中身 | 分担 | 見積り |
|---|---|---|---|
| M | z が働く 80 問 × 種 1・2 × {def, z}、600 秒・4 スレッド | 21 (K = 0〜20) | 各 1.67〜1.75 時間 |
| P | MIPLIB の 58 問 × 種 1〜3 × {def, z}、60 秒・4 スレッド | 2 (K = 0〜1) | 各 1.41 時間 |

見積りは analysis/vsscip_20261117 の def の時間 (種 1、解けなければ上限) + 1 回ごとに M 30 秒・P 8 秒。同じ (問題, 種) の 2 組は同じ分担で続けて解く。

## 3. 回し方・集計

- ビルドは a7e9d28e8 以降 (新しい MPS 読み込み入り) の先端。回したコミットをここに書き足すこと。
  - M 7: 23888f312 (wheel を site-packages に入れて回した)。
  - M 0: ビルド 9711bc9d6 (作業ツリー 972c46d06)。
  - P 0: ビルド 9711bc9d6 (build.sh の wheel)、作業ツリー e05d73651 (cc50efa22 以降)。174 行・crash 0。
  - P 1: ビルド c5c5c3e31 (build.sh の wheel)、作業ツリー 4f219fff2 (cc50efa22 以降)。174 行・crash 0。
  - M 4: ビルド 2e6c79119 (build.sh の wheel)、作業ツリー f9334a077 (cc50efa22 以降)。14 行・crash 0。
  - M 2: ビルド 9711bc9d6 (build.sh の wheel)、作業ツリー 441cc7713 (cc50efa22 以降)。10 行・crash 0。
  - M 1: ビルド 820b1e9d3 (build.sh の wheel、src・python は 441cc7713 と同じ)、作業ツリー 441cc7713 (cc50efa22 以降)。10 行・crash 0。
  - M 3: ビルド 9711bc9d6 (build.sh の wheel)、作業ツリー 1509d60e8 (cc50efa22 以降)。10 行・crash 0。
  - M 5: ビルド 9711bc9d6 (build.sh の wheel)、作業ツリー 867d487fb (cc50efa22 以降)。14 行・crash 0。
  - M 6: ビルド 2e6c79119 (build.sh の wheel)、作業ツリー d855dfead (cc50efa22 以降)。16 行・crash 0。
  - M 11: ビルド f2852d652 (build.sh の wheel)、作業ツリー e3c0ccf2c (cc50efa22 以降)。16 行・crash 0。
  - M 8: ビルド da1e60d2c (pip install -e .)、作業ツリー bda63775c (cc50efa22 以降、src はビルドと同じ)。16 行・crash 0。
  - M 10: ビルド 9711bc9d6 (build.sh の wheel)、作業ツリー d138ae23c (cc50efa22 以降、src・python はビルドと同じ)。16 行・crash 0 (10 行で一度止めて続きから)。
  - M 12: ビルド 8e9e75138 (pip install -e .)、作業ツリーも 8e9e75138 (cc50efa22 以降)。16 行・crash 0。
  - M 9: ビルド a5aa1bc31 (build.sh の wheel)、作業ツリー 246d9c201 (cc50efa22 以降)。16 行・crash 0。
  - M 14: ビルド 9711bc9d6 (build.sh の wheel)、作業ツリー d8fa7935c (cc50efa22 以降)。16 行・crash 0。
- 回す: `cd analysis/cutmixz_20261120 && python run.py M K` (K = 0〜20)、`python run.py P K` (K = 0〜1)。res_M_sK.jsonl・res_P_sK.jsonl に足す (途中から再開できる)。
- 集計: `python summ.py M def z > summ_M.txt`、`python summ.py P def z > summ_P.txt` (SGM・区間・解けた数・食い違い・全種で 1.25 倍超の遅れ)。
  P は z が働かない問題も入るので、働く問題だけの比べは zscan と同じ調べを P の問題にもしてから分ける。
- 回したコミット: M 13 はビルド 1607f5185 (作業ツリー 6ce0e7991)。
