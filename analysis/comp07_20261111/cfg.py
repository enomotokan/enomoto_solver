"""COMP07 (2026-11-11) の設定。def は環境変数なし (ビルドごとの既定)。
今の既定 (f50eba16) = cand1108zd + DRCR (ENOMOTO_MIP_FOLD) + SYMDYN=f。cand1108zd の中身は analysis/halve_submip_20261030/run.py の CFGS。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("hs_run", Path(__file__).resolve().parents[1] / "halve_submip_20261030" / "run.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
HS = _m.CFGS

PRE = HS["precand1108zd"]  # cand1108zd の 20 個を元に戻す形
CFGS = {
    "def": {},
    "nofold": {"ENOMOTO_MIP_FOLD": "0"},
    "nosymdyn": {"ENOMOTO_MIP_SYMDYN": "0"},
    "zh": {"ENOMOTO_MIP_NO_ZEROHALF": "0"},                      # zerohalf を戻す
    "ebiR": {"ENOMOTO_MIP_PRECHAIN": "ebiR"},                    # RESTCHAIN: FJ・Local-MIP の連鎖を木に入るまで続ける
    "ebi": {"ENOMOTO_MIP_PRECHAIN": "ebi"},                      # PRECHAIN: 前処理の間から FJ・Local-MIP (根のカットのループの終わりまで)
    "ebiRK": {"ENOMOTO_MIP_PRECHAIN": "ebiRK"},                  # COMP07 の試作: R の延長を、主の暫定解が連鎖の解だけの間に限る
    "fjkeep": {"ENOMOTO_MIP_FJKEEP": "1"},                       # COMP07 の試作: 根の FJ のスレッドを根のヒューリスティクスの終わりまで続ける
    "pre": dict(PRE, ENOMOTO_MIP_FOLD="0", ENOMOTO_MIP_SYMDYN="0"),  # 今のコードで res6 の既定に寄せた形
}
# cand1108zd の部品を 1 つずつ元に戻す形 (drop_<名>)。組の部品はまとめて戻す
GROUPS = {
    "scale": ["ENOMOTO_SIMPLEX_SCALE", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE", "ENOMOTO_SIMPLEX_HARRIS2"],
    "mpsep": ["ENOMOTO_MIP_MPSEP", "ENOMOTO_T_MIP_MPSEP_NV", "ENOMOTO_T_MIP_MPSEP_EXTRA"],
    "mushagg": ["ENOMOTO_MIP_MUSHAGG"],
    "prr": ["ENOMOTO_MIP_PROBE_REPRESOLVE"],
    "subfix": ["ENOMOTO_MIP_SUBFIX"],
    "phgb": ["ENOMOTO_MIP_NODELP_PERTURB", "ENOMOTO_MIP_BASIS_NB"],
    "pbg": ["ENOMOTO_MIP_PROBE_BASIS_GUARD"],
    "hnfpre": ["ENOMOTO_MIP_HNFPRE"],
    "cutlpcap": ["ENOMOTO_MIP_CUTLPCAP"],
    "cmirdelta": ["ENOMOTO_MIP_CMIRDELTA"],
    "lpscale": ["ENOMOTO_MIP_LPSCALE", "ENOMOTO_T_MIP_LPSCALE_MIN_RANGE"],
    "zh": ["ENOMOTO_MIP_NO_ZEROHALF"],
    "dse": ["ENOMOTO_MIP_DSE_INIT"],
}
assert sorted(sum(GROUPS.values(), [])) == sorted(PRE), sorted(set(PRE) ^ set(sum(GROUPS.values(), [])))
for g, ks in GROUPS.items():
    CFGS[f"drop_{g}"] = {k: PRE[k] for k in ks}
