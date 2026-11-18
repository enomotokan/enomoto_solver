"""段階 2 の A (README の「段階 2 の分担表」): clique カットのシグモイドの二極化の確かめ。
def = 既定 (環境変数なし、ENOMOTO_MIP_CLIQUE_CUTS の既定は sigmoid)、fb = 以前の既定 (fallback)、sa = 適応の形 sigmoid_a (試作)。"""
CFGS = {"def": {}, "fb": {"ENOMOTO_MIP_CLIQUE_CUTS": "fallback"}, "sa": {"ENOMOTO_MIP_CLIQUE_CUTS": "sigmoid_a"}}
