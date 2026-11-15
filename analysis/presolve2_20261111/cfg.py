"""PRESOLVE2 (2026-11-12) の設定。def = 既定 (環境変数なし)。
spl  = 前処理の最後に HiGHS 式の疎化 (既存の試作 ENOMOTO_MIP_SPARSIFY_LATE。kasai・puriri・mzzv の Sparsify の差)
enum = 列挙の前処理を全部の候補の行に (既存の試作 ENOMOTO_MIP_HO3_ENUM。net12・puriri の Enumeration の差)
deq  = 双対の議論で行を等式にする (このセッションの試作 ENOMOTO_MIP_DUALEQ。neos859080 の差)"""
CFGS = {
    "def": {},
    "spl": {"ENOMOTO_MIP_SPARSIFY_LATE": "1"},
    "enum": {"ENOMOTO_MIP_HO3_ENUM": "1"},
    "deq": {"ENOMOTO_MIP_DUALEQ": "1"},
}
LOGENV = {"ENOMOTO_MIP_LOG": "1"}
CFGS["se"] = {**CFGS["spl"], **CFGS["enum"]}                # spl + enum (対象の問題の 600 秒の回だけ)
CFGS["sed"] = {**CFGS["spl"], **CFGS["enum"], **CFGS["deq"]}
