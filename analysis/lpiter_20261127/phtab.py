"""当方 (ENOMOTO_PROF_PHASES_EXT の出力) と HiGHS (hphase.py の JSON) の 1 反復の内訳を同じ項目に寄せて表にする。
使い方: python phtab.py <当方のログ> <HiGHS の JSON の行を含むファイル> [LP 名]"""
import json, re, sys
MAP = {  # 項目: (当方の段階, HiGHS の時計)
    "CHUZR": (["chuzr"], ["CHUZR_DUAL", "CHUZR1", "CHUZR2"]),
    "BTRAN": (["btran(rho_p)"], ["BTRAN"]),
    "PRICE": (["price"], ["PRICE"]),
    "CHUZC・BFRT": (["chuzc1", "bfrt"], ["CHUZC0", "CHUZC1", "CHUZC2", "CHUZC3", "CHUZC4", "CHUZC5", "FTRAN_BFRT"]),
    "FTRAN (列 + DSE)": (["ftran"], ["FTRAN", "FTRAN_DSE"]),
    "x_B 更新": (["xb_update"], ["UPDATE_PRIMAL"]),
    "DSE 更新": (["dse_update"], ["UPDATE_DSE_WEIGHT"]),
    "双対の更新": (["dual_update"], ["UPDATE_DUAL"]),
    "LU の更新": (["ft_update"], ["UPDATE_FACTOR", "UPDATE_PIVOTS", "UPDATE_MATRIX"]),
    "再分解・作り直し": (["refactor"], ["INVERT", "COMPUTE_DUAL", "CORRECT_DUAL", "COMPUTE_PRIMAL", "COLLECT_PR_IFS", "COMPUTE_PR_IFS", "COMPUTE_DU_IFS",
                                 "COMPUTE_DU_OBJ", "COMPUTE_PR_OBJ", "REPORT_REBUILD", "PERM_WT", "BTRAN_FULL", "PRICE_FULL"]),
}


def ours(path):
    ph, it = {}, None
    for l in open(path):
        m = re.match(r"\s+([a-z_0-9()]+)\s+([\d.]+)ms\s+[\d.]+% of wall", l)
        if m: ph[m.group(1)] = ph.get(m.group(1), 0.0) + float(m.group(2)) / 1e3
        m = re.match(r"PROF_PHASES_EXT wall=([\d.]+)ms iters=(\d+)", l)
        if m: ph["_wall"] = ph.get("_wall", 0.0) + float(m.group(1)) / 1e3; it = (it or 0) + int(m.group(2))
    return ph, it


def table(o, oi, h, hi, name=""):
    print(f"| {name} 項目 | 当方 µs/反復 | HiGHS µs/反復 | 倍 |\n|---|---:|---:|---:|")
    so = sh = 0.0
    for k, (a, b) in MAP.items():
        x = 1e6 * sum(o.get(t, 0.0) for t in a) / oi; y = 1e6 * sum(h.get(t, 0.0) for t in b) / hi; so += x; sh += y
        print(f"| {k} | {x:.1f} | {y:.1f} | {x / y if y else float('nan'):.2f} |")
    print(f"| 合計 (計時した段階) | {so:.1f} | {sh:.1f} | {so / sh:.2f} |")
    print(f"| 反復 | {oi} | {hi} | {oi / hi:.2f} |")


if __name__ == "__main__":
    o, oi = ours(sys.argv[1])
    hl = [json.loads(l) for l in open(sys.argv[2]) if l.startswith("{")]
    hl = [r for r in hl if r.get("timer") and (len(sys.argv) < 4 or r.get("lp") == sys.argv[3])]
    table(o, oi, hl[-1]["phases"], hl[-1]["iters"], sys.argv[3] if len(sys.argv) > 3 else "")
