"""クリークのカットの自動の判定の見直しの確かめ (analysis/rootgap_20261116/README.md の 4 節)。
- c18: CAND1118 (analysis/cand1118_20261116/cfg.py。クリークのカットは既定の `fallback`)
- c18_clqroot: + ENOMOTO_MIP_CLIQUE_CUTS=root (根でいつも回す)
- c18_clqmarg: + ENOMOTO_T_MIP_CLIQUE_FB_MARGINAL=5 + ENOMOTO_T_MIP_CLIQUE_FB_SECOND_ROUNDS=10 (fallback の判定を、ループの最後の 5 ラウンドの伸びと比べる
  形にし、使うときに続ける 2 回目のループを 10 ラウンドまでにする。どちらも既定 0 の試作の鍵)"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1118", Path(__file__).resolve().parents[2] / "cand1118_20261116" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
C18 = dict(_m.CAND1118)
CFGS = {"c18": C18, "c18_clqroot": {**C18, "ENOMOTO_MIP_CLIQUE_CUTS": "root"},
        "c18_clqmarg": {**C18, "ENOMOTO_T_MIP_CLIQUE_FB_MARGINAL": "5", "ENOMOTO_T_MIP_CLIQUE_FB_SECOND_ROUNDS": "10"}}
