"""試作の組 (run.py の 5 つ目の引数の名前 → cand1114 に足す環境変数)。base は cand1114 だけ (試作と同じ build で測り直す基準)。"""
CFGS = {
    "base": {},
    "s": {"ENOMOTO_MIP_ROBUST": "s"},
    "p": {"ENOMOTO_MIP_ROBUST": "p"},
    "sp": {"ENOMOTO_MIP_ROBUST": "sp"},
}
CFGS.update({"f": {"ENOMOTO_MIP_ROBUST": "f"}, "spf": {"ENOMOTO_MIP_ROBUST": "spf"}})
