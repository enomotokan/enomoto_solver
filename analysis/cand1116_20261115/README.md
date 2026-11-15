# cand1116 の確かめ (2026-11-15)

設定は cfg.py (cand1114 / cand1116 / cand1116f)。分担表は plan.py (python plan.py 16 6)。
- P: 58 問 × 種 1〜4 × 3 設定、60 秒、6 分担 (各約 1 時間): `python run.py P K` (K = 0〜5)
- M: Mittelmann 48 問 × 種 1・2 × 3 設定、600 秒、16 分担 (各約 1.9 時間): `python run.py M K` (K = 0〜15)
いずれも 4 スレッド、ソルバーは同時に 1 つ。途中から再開できる。集計は `python summ.py P` / `python summ.py M`。
準備: pip install maturin highspy==1.15.1 && pip install -e . 、python analysis/cbcimpl_20261029/probs.py (問題の取得)。
