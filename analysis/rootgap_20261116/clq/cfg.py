"""クリークのカットの自動の判定の見直しの確かめ (analysis/rootgap_20261116/README.md の 4 節)。
- c18: CAND1118 (analysis/cand1118_20261116/cfg.py。クリークのカットは既定の `fallback`)
- c18_clqroot: + ENOMOTO_MIP_CLIQUE_CUTS=root (根でいつも回す)
- c18_clqmarg: + ENOMOTO_T_MIP_CLIQUE_FB_MARGINAL=5 + ENOMOTO_T_MIP_CLIQUE_FB_SECOND_ROUNDS=10 (fallback の判定を、ループの最後の 5 ラウンドの伸びと比べる
  形にし、使うときに続ける 2 回目のループを 10 ラウンドまでにする。どちらも既定 0 の試作の鍵)
- c18_sig: + ENOMOTO_MIP_CLIQUE_CUTS=sigmoid、a = 1.8・b = 0.41 (README の 2 節。根の 1 ラウンドの clique カットを round(f x 100) 本までにする。
  f = 1 / (1 + exp(-a (s - b)))。a・b は sigfit.py の P・M の時間の比 root / c18 での回帰)
- c18_siga: c18_sig + sigmoid_a (根の 3 ラウンド目に clique カット 1 回の伸びと直前のラウンドの伸びの比で f を決め直す、β = 1)"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1118", Path(__file__).resolve().parents[2] / "cand1118_20261116" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
C18 = dict(_m.CAND1118)
CFGS = {"c18": C18, "c18_clqroot": {**C18, "ENOMOTO_MIP_CLIQUE_CUTS": "root"},
        "c18_clqmarg": {**C18, "ENOMOTO_T_MIP_CLIQUE_FB_MARGINAL": "5", "ENOMOTO_T_MIP_CLIQUE_FB_SECOND_ROUNDS": "10"}}
SIG_AB = {"ENOMOTO_T_MIP_CLIQUE_SIG_A": "1.8", "ENOMOTO_T_MIP_CLIQUE_SIG_B": "0.41"}
CFGS |= {"c18_parint": {**C18, "ENOMOTO_PRESOLVE_PARALLEL_INT": "1"}, "c18_twins": {**C18, "ENOMOTO_MIP_SYM_TWINS": "1"}}  # 双子の併合 (rootgap README の 5 節、P5・M5)
CFGS |= {"c18_sig": {**C18, "ENOMOTO_MIP_CLIQUE_CUTS": "sigmoid", **SIG_AB}, "c18_siga": {**C18, "ENOMOTO_MIP_CLIQUE_CUTS": "sigmoid_a", **SIG_AB}}
