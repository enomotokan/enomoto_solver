"""CLQPRB (2026-11-12) の設定。def = 既定 (環境変数なし)。
clq = このセッションの試作 ENOMOTO_MIP_CLQPRB=1 (プロービングで学んだ含意を伝播に使い、含意の上限・予算・時間を広げ、前処理のかけ直しの
      後のプロービングに含意を引き継ぐ。analysis/clqprb_20261112.md)"""
CFGS = {
    "def": {},
    "clq": {"ENOMOTO_MIP_CLQPRB": "1"},
}
LOGENV = {"ENOMOTO_MIP_LOG": "1"}
