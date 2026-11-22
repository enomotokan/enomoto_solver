"""cmirgen の設定 (analysis/cmirgen_20261122)。def: 今の既定。試作 ENOMOTO_MIP_CMIRGEN の f (f0 の上限 0.999)・d (SCIP の δ の候補)・fd (両方)・u (f0 の上限なし、1 - f0 <= 1e-6 だけ除く)。"""
CFGS = {"def": {}, "f": {"ENOMOTO_MIP_CMIRGEN": "f"}, "d": {"ENOMOTO_MIP_CMIRGEN": "d"}, "fd": {"ENOMOTO_MIP_CMIRGEN": "fd"},
        # u: f0 の決め打ちの上限を外し、1 - f0 <= FEASTOL (1e-6) だけ除く (利用者の案)。fdiag は f と同じ設定で診断だけ取り直す回
        "u": {"ENOMOTO_MIP_CMIRGEN": "u"}, "fdiag": {"ENOMOTO_MIP_CMIRGEN": "f"}}
