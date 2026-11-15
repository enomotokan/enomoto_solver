"""run.py のログ (logs/<設定>/<問題>_r<回>.log) から表を作る。

使い方: python analysis/singular_fix_c_20261102/table.py > analysis/singular_fix_c_20261102/tables.md
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

CFGS = ["base", "fix"]
TARGETS = ["ns1116954", "mzzv42z", "neos-960392", "physiciansched6-2", "neos-827175"]
TL = 60.0


def parse(path):
    d = {"diag": {}, "root": None, "restart_lp": [], "fix": []}
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
        elif l.startswith("MIP: root LP basis fix"):
            d["fix"].append(l)
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


def main():
    D = load()
    print("# ENOMOTO_MIP_ROOT_BASIS_FIX の計測\n")
    # 1. 5 問の根
    print("## 1. 根の LP が重い 5 問 (回ごと)\n")
    print("| 問題 | 設定 | 回 | 結果 | 時間 | 根の LP (反復 / 秒) | root_probe (LP / 特異 / 反復 / 秒) | restart (LP / 特異 / 反復 / 秒) | 再スタートの後の根の LP |")
    print("|---|---|---|---|---|---|---|---|---|")
    for p in TARGETS:
        for c in CFGS:
            for r, d in sorted(D.get((c, p), {}).items()):
                res = d.get("res", {})
                def g(k):
                    x = d["diag"].get(k)
                    if not x:
                        return "–"
                    return f"{int(x['n'])} / {int(x['n_noeff'])} / {int(x['it_mis'] + x['it_ok'])} / {x['t_mis'] + x['t_ok']:.1f}"
                root = d["root"]
                rs = "–" if root is None else f"{root[0] if root[0] is not None else '上限'} / {root[1]:.1f}"
                rl = ", ".join(f"{s} {i} / {t:.1f}" for s, i, t in d["restart_lp"]) or "–"
                st = res.get("status")
                print(f"| {p} | {c} | {r} | {st} {res.get('objective')} | {t_of(d):.1f} | {rs} | {g('root_probe')} | {g('restart')} | {rl} |")
    # 2. 58 問の NB_DIAG の合計 (回 2)
    print("\n## 2. 58 問 (回 2) の root_probe・restart の合計\n")
    print("| 設定 | 場所 | 問題 | LP | 特異で捨てた LP | 反復 | 時間 |")
    print("|---|---|---|---|---|---|---|")
    for c in CFGS:
        for loc in ["root_probe", "restart", "slack_retry"]:
            n = noeff = it = t = 0
            k = 0
            for p in probs.PROBS:
                d = D.get((c, p), {}).get(2)
                if not d or loc not in d["diag"]:
                    continue
                x = d["diag"][loc]
                k += 1
                n += x["n"]
                noeff += x["n_noeff"]
                it += x["it_mis"] + x["it_ok"]
                t += x["t_mis"] + x["t_ok"]
            print(f"| {c} | {loc} | {k} | {int(n)} | {int(noeff)} | {int(it)} | {t:.1f} 秒 |")
    # 3. 58 問の全体
    print("\n## 3. 58 問 (回 2) の全体 (4 スレッド・60 秒、SGM はずらし 10 秒、解けなかった問題は 60 秒)\n")
    rows = []
    ts = {c: [] for c in CFGS}
    solved = {c: 0 for c in CFGS}
    for p in probs.PROBS:
        ds = {c: D.get((c, p), {}).get(2) for c in CFGS}
        if not all(ds.values()):
            continue
        for c in CFGS:
            ts[c].append(t_of(ds[c]))
            solved[c] += ds[c].get("res", {}).get("status") == "optimal"
        rows.append((p, ds))
    if rows:
        print("| 設定 | 問題 | 解けた | SGM |")
        print("|---|---|---|---|")
        for c in CFGS:
            print(f"| {c} | {len(ts[c])} | {solved[c]} | {sgm(ts[c]):.2f} 秒 |")
        print(f"\n比 (fix / base): {(sgm(ts['fix']) + 10) / (sgm(ts['base']) + 10) - 1:+.1%} (ずらした幾何平均)\n")
        print("### 問題ごと (root_probe か restart のある問題と、時間が 10% 以上かつ 1 秒以上違う問題)\n")
        print("| 問題 | base 時間 | fix 時間 | base 根の LP | fix 根の LP | base root_probe/restart | fix root_probe/restart | fix の基底 |")
        print("|---|---|---|---|---|---|---|---|")
        for p, ds in rows:
            tb, tf = t_of(ds["base"]), t_of(ds["fix"])
            has = any(k in ds[c]["diag"] for c in CFGS for k in ("root_probe", "restart"))
            if not has and not (abs(tb - tf) >= 1.0 and abs(tb - tf) >= 0.1 * max(tb, tf)):
                continue
            def rp(d):
                out = []
                for k in ("root_probe", "restart"):
                    x = d["diag"].get(k)
                    if x:
                        out.append(f"{k[:5]} {int(x['n_noeff'])}/{int(x['n'])} 特異 {int(x['it_mis'] + x['it_ok'])} 反復 {x['t_mis'] + x['t_ok']:.1f} 秒")
                return "; ".join(out) or "–"
            def root(d):
                r = d["root"]
                return "–" if r is None else f"{r[0] if r[0] is not None else '上限'} / {r[1]:.1f}"
            fx = "; ".join(re.sub(r".*RootBasisFixStats \{ (.*?) \}.*?(used|not used)?.*", r"\1 \2", l).replace("singular: false", "").strip() for l in ds["fix"]["fix"])
            print(f"| {p} | {tb:.1f} | {tf:.1f} | {root(ds['base'])} | {root(ds['fix'])} | {rp(ds['base'])} | {rp(ds['fix'])} | {fx or '–'} |")


main()
