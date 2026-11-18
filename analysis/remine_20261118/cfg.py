"""remine の短い計測の設定 (README の上位 5 件の 5・3)。def = 今の既定 (環境変数なし)。
- g4off: ENOMOTO_MIP_GAP4=0 (既定の rocn を外す。src/mip/solver.rs の gap4)
- lucg: ENOMOTO_SIMPLEX_LUCLOCK=g (eta の増え方だけで分解し直す試作。analysis/luclock_20261111.md)"""
CFGS = {"def": {}, "g4off": {"ENOMOTO_MIP_GAP4": "0"}, "lucg": {"ENOMOTO_SIMPLEX_LUCLOCK": "g"}}
