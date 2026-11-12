"""CLQPRB (2026-11-12) の設定。def = 既定 (環境変数なし)。
clq = このセッションの試作 ENOMOTO_MIP_CLQPRB=1 (プロービングで学んだ含意を伝播に使い、含意の上限・予算・時間を広げ、前処理のかけ直しの
      後のプロービングに含意を引き継ぐ。analysis/clqprb_20261112.md)"""
CFGS = {
    "def": {},
    "clq": {"ENOMOTO_MIP_CLQPRB": "1"},
    # clqd = clq + 何も固定・締め付けできなければ増やした含意を捨てる (air03 の対策の試し)
    "clqd": {"ENOMOTO_MIP_CLQPRB": "d"},
    # clqk = clq + 既存の試作 GAP4 の k (locks の固定に含意も使い、根で実行不能なら固定率を半分にしてやり直す。puriri の対策の試し)
    "clqk": {"ENOMOTO_MIP_CLQPRB": "1", "ENOMOTO_MIP_GAP4": "k"},
    # clqa = clq + 試作 CLQACT (行の活動量のクリーク分割で係数・境界を締める、analysis/clqprb_20261112.md §12)
    "clqa": {"ENOMOTO_MIP_CLQPRB": "1", "ENOMOTO_MIP_CLQACT": "1"},
}
LOGENV = {"ENOMOTO_MIP_LOG": "1"}
