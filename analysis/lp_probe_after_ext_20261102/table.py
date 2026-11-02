"""run.py のログ (logs/<設定>/<問題>_r<回>.log) から表を作る。

使い方: python analysis/lp_probe_after_ext_20261102/table.py > analysis/lp_probe_after_ext_20261102/tables.md
"""
import json
import math
import re
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "analysis" / "cbcimpl_20261029"))
import probs  # noqa: E402

CFGS = ["base", "ext", "extr", "base_nofj", "ext_nofj", "extr_nofj"]
TARGETS = ["ns1116954", "mzzv42z", "neos-960392", "physiciansched6-2", "neos-827175", "hypothyroid-k1"]
TL = 60.0
REPS58 = (10, 11)


def parse(path):
    d = {"diag": {}, "root": None, "restart_lp": [], "fixed": [], "basis": None, "lp_probe": None, "ext": None}
    for l in path.read_text().splitlines():
        if l.startswith("RESULT "):
            d["res"] = json.loads(l[7:])
        elif l.startswith("NB_DIAG main "):
            f = l.split()
            kv = dict(x.split("=", 1) for x in f[3:])
            d["diag"][f[2]] = {k: float(v) for k, v in kv.items()}
        elif d["root"] is None and (m := re.match(r"MIP: root LP [-0-9.e]+ \((\d+) iters, ([0-9.]+)s\)", l)):
            d["root"] = (int(m[1]), float(m[2]))
        elif d["root"] is None and (m := re.match(r"MIP: root LP stopped at the time limit \(([0-9.]+)s", l)):
            d["root"] = (None, float(m[1]))
        elif m := re.match(r"MIP: root LP after the restart: (\w+), (\d+) iters \(([0-9.]+)s\)", l):
            d["restart_lp"].append((m[1], int(m[2]), float(m[3])))
        elif m := re.match(r"MIP: LP probing: tried (\d+) columns, fixed (\d+)", l):
            d["fixed"].append(int(m[2]))
        elif m := re.match(r"MIP: LP probing: root LP ([0-9.]+)s \((\d+) iters", l):
            # 前処理の LP (最初の LP) の時間・反復 (再スタートの後はない)
            d["lp_probe"] = (float(m[1]), int(m[2]))
        elif l.startswith("MIP: root LP basis from the presolve LP probing: ") and d["basis"] is None:
            d["basis"] = l.split(": ", 1)[1]
        elif d["ext"] is None and (m := re.match(r"MIP: clique extension: (\d+) rows extended, (\d+) nonzeros added, \d+ columns fixed, (\d+) duplicate and (\d+) dominated", l)):
            d["ext"] = (int(m[1]), int(m[2]), int(m[3]) + int(m[4]))
    return d


def load():
    out = defaultdict(dict)
    for c in CFGS:
        for f in sorted((HERE / "logs" / c).glob("*.log")):
            p, r = f.stem.rsplit("_r", 1)
            out[(c, p)][int(r)] = parse(f)
    return out


def t_of(d):
    r = d.get("res", {})
    t = r.get("time")
    if r.get("status") != "optimal" or t is None:
        return TL
    return min(float(t), TL)


def sgm(ts, s=10.0):
    return math.exp(sum(math.log(t + s) for t in ts) / len(ts)) - s


def diag(d, k):
    x = d["diag"].get(k)
    if not x:
        return "–"
    return f"{int(x['n'])} / {int(x['n_noeff'])} / {int(x['it_mis'] + x['it_ok'])} / {x['t_mis'] + x['t_ok']:.1f}"


def root(d):
    r = d["root"]
    return "–" if r is None else f"{r[0] if r[0] is not None else '上限'} / {r[1]:.1f}"


def short_basis(b):
    return {"used (with rows)": "行も", "used (with rows; the added rows basic)": "行も (末尾は基底)", "used (columns only)": "列だけ", "none": "なし"}.get(b, b or "–")


def main():
    D = load()
    print("# ENOMOTO_MIP_LP_PROBE_AFTER_EXT の計測\n")
    print("## 1. 対象の問題 (回ごと、4 スレッド・60 秒)\n")
    print("root_probe・restart の欄は NB_DIAG の (LP / 特異で捨てた LP / 反復 / 秒)。固定は `MIP: LP probing: ... fixed N` の N (前処理のかけ直しの中の分も並べる)。\n")
    print("| 問題 | 設定 | 回 | 結果 | 時間 | 前処理の LP (秒 / 反復) | 固定 | 根に渡した基底 | 根の LP (反復 / 秒) | root_probe | restart |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for p in TARGETS:
        for c in CFGS:
            for r, d in sorted(D.get((c, p), {}).items()):
                if r >= 10:
                    continue
                res = d.get("res", {})
                lpp = "–" if d["lp_probe"] is None else f"{d['lp_probe'][0]:.1f} / {d['lp_probe'][1]}"
                fx = "+".join(map(str, d["fixed"])) or "0"
                print(f"| {p} | {c} | {r} | {res.get('status')} {res.get('objective')} | {t_of(d):.1f} | {lpp} | {fx} | {short_basis(d['basis'])} | {root(d)} | {diag(d, 'root_probe')} | {diag(d, 'restart')} |")
    # 2. 58 問
    have = [c for c in CFGS if any(r in REPS58 for (cc, p), v in D.items() if cc == c for r in v)]
    if len(have) < 2:
        return
    print("\n## 2. 58 問の NB_DIAG の合計 (回ごと)\n")
    print("| 設定 | 回 | 場所 | 問題 | LP | 特異で捨てた LP | 反復 | 時間 |")
    print("|---|---|---|---|---|---|---|---|")
    for c in have:
        for rep in REPS58:
            for loc in ["root_probe", "restart"]:
                n = noeff = it = t = 0
                k = 0
                for p in probs.PROBS:
                    d = D.get((c, p), {}).get(rep)
                    if not d or loc not in d["diag"]:
                        continue
                    x = d["diag"][loc]
                    k += 1
                    n += x["n"]
                    noeff += x["n_noeff"]
                    it += x["it_mis"] + x["it_ok"]
                    t += x["t_mis"] + x["t_ok"]
                print(f"| {c} | {rep} | {loc} | {k} | {int(n)} | {int(noeff)} | {int(it)} | {t:.1f} 秒 |")
    print("\n## 3. 58 問の全体 (4 スレッド・60 秒、SGM はずらし 10 秒、解けなかった問題は 60 秒)\n")
    print("| 設定 | 回 | 問題 | 解けた | SGM |")
    print("|---|---|---|---|---|")
    full = [p for p in probs.PROBS if all(D.get((c, p), {}).get(r) for c in have for r in REPS58)]
    tsum = {}
    for c in have:
        for rep in REPS58:
            ts = [t_of(D[(c, p)][rep]) for p in full]
            sv = sum(D[(c, p)][rep].get("res", {}).get("status") == "optimal" for p in full)
            tsum[(c, rep)] = ts
            print(f"| {c} | {rep} | {len(ts)} | {sv} | {sgm(ts):.2f} 秒 |")
        ts2 = tsum[(c, REPS58[0])] + tsum[(c, REPS58[1])]
        print(f"| {c} | 2 回 | {len(ts2)} | {sum(D[(c, p)][r].get('res', {}).get('status') == 'optimal' for p in full for r in REPS58)} | {sgm(ts2):.2f} 秒 |")
    if "base" in have:
        for c in have:
            if c == "base":
                continue
            a = tsum[("base", REPS58[0])] + tsum[("base", REPS58[1])]
            b = tsum[(c, REPS58[0])] + tsum[(c, REPS58[1])]
            print(f"\n比 ({c} / base、2 回): {(sgm(b) + 10) / (sgm(a) + 10) - 1:+.1%} (ずらした幾何平均)")
    print("\n### 問題ごと (前処理の LP の固定・基底のある問題と、時間の平均が 10% 以上かつ 1 秒以上違う問題)\n")
    hdr = " | ".join(f"{c} 時間 | {c} 固定 | {c} 根の LP | {c} 基底" for c in have)
    print(f"| 問題 | {hdr} |")
    print("|---|" + "---|" * (4 * len(have)))
    for p in full:
        avg = {c: sum(t_of(D[(c, p)][r]) for r in REPS58) / len(REPS58) for c in have}
        interesting = any(D[(c, p)][r]["fixed"] or D[(c, p)][r]["basis"] not in (None, "none") for c in have for r in REPS58)
        diff = max(avg.values()) - min(avg.values())
        if not interesting and not (diff >= 1.0 and diff >= 0.1 * max(avg.values())):
            continue
        cells = []
        for c in have:
            ds = [D[(c, p)][r] for r in REPS58]
            cells.append("・".join(f"{t_of(d):.1f}" for d in ds))
            cells.append("・".join("+".join(map(str, d["fixed"])) or "0" for d in ds))
            cells.append("・".join(root(d) for d in ds))
            cells.append("・".join(short_basis(d["basis"]) for d in ds))
        print(f"| {p} | " + " | ".join(cells) + " |")


main()
