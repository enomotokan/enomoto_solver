# 0.1.9 の既定と HiGHS 1.15.1 の比べ (2026-11-17、利用者の依頼)

既定 (def、環境変数なし。0.1.9 = cand1119 + カットプールの共有 + 空の行の許容の直し) と HiGHS 1.15.1 (highs) を、同じ機械・同じ種で対にして比べる。
台本は cand1118_20261116 の写し (設定を def・highs の 2 つに)。4 スレッド、ソルバーは同時に 1 つ。途中から再開できる。

- M: Mittelmann 48 問 (cand1118 の M と同じ) × 種 1・2 × {def, highs}、600 秒、12 分担 (見積り各約 1.5 時間): `python run.py M K` (K = 0〜11)
- P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, highs}、60 秒、4 分担 (見積り各約 0.75 時間): `python run.py P K` (K = 0〜3)
- 集計: `python summ.py M > summ_M.txt`、`python summ.py P > summ_P.txt` (SGM shift 1・問題単位のブートストラップの 95% CI、解けた数、食い違い、最大 RSS)。
- 準備: pip install maturin highspy==1.15.1 && pip install -e . (この台本のコミット以降でビルド)。
- A (利用者の依頼「Mittelmann は全部」): Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) のうち M の 48 問以外の 138 問 × 種 1 × {def, highs}、600 秒、
  26 分担 (見積り各 1.4〜1.75 時間、HiGHS の 60 秒の回で解けない問題は 600 秒と見積もる): `python planA.py 26` → `python run.py A K` (K = 0〜25)。
  186 問の集計は `python summ.py MA > summ_MA.txt` (M の 48 問は種 1・2、A の 138 問は種 1)。
