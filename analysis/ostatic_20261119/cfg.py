"""試作の設定 (analysis/ostatic_20261119)。def = 今の既定。
orr = 既定 (動的な orbitopal fixing + ORBPP の静的なオービトープ) の静的なオービトープの固定に理由を付ける (ENOMOTO_MIP_ORBITOPE_REASON=1)。
ostr = 全てのオービトープを静的に扱い (ENOMOTO_MIP_ORBITOPE_STATIC)、固定に理由を付ける (値の r)。
st = 全てのオービトープを静的に扱うだけ (理由なし、ostr の比べ用)。"""
CFGS = {"def": {}, "orr": {"ENOMOTO_MIP_ORBITOPE_REASON": "1"}, "ostr": {"ENOMOTO_MIP_ORBITOPE_STATIC": "r"}, "st": {"ENOMOTO_MIP_ORBITOPE_STATIC": "1"}}
