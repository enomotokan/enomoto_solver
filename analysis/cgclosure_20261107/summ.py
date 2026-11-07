"""表を作る (2026-11-07): 閉じたギャップの割合 = (下界 - LP) / (最適値 - LP) (最小化の向きで)。
LP は前処理なしの元の模型の LP (cgclosure.py の lp)。最適値は MIPLIB 2017 の solu (/tmp/miplib.solu、v29)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[0] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
PROBS = PROBS + ["mushroom-best"]
opt = {}
for l in open(D / "miplib2017-v29.solu"):
    p = l.split()
    if len(p) >= 3 and p[0] == "=opt=":
        opt[p[1]] = float(p[2])
roots = {}
for l in (D / "roots.jsonl").read_text().splitlines():
    r = json.loads(l); roots[(r["problem"], r["kind"])] = r
clo = {}
fn = sys.argv[1] if len(sys.argv) > 1 else "closures.jsonl"
for f_ in [fn] + sys.argv[2:]:  # 後のファイルが前のものを上書きする (やり直し)
    for l in (D / f_).read_text().splitlines():
        r = json.loads(l); clo[(r["name"], r["mode"])] = r
MODES = ["cg", "mod2", "modk"]


def pct(b, lp, z, sense):
    if b is None or lp is None or z is None:
        return None
    b, z = b * sense, z * sense
    g = z - lp
    if abs(g) <= 1e-6 * max(1.0, abs(z)):
        return None
    return 100.0 * (b - lp) / g


def f(v):
    return "—" if v is None else f"{v:.1f}"


rows = []
print("| 問題 | LP (最小化の向き) | 最適値 | ENOMOTO 既定の根 | HiGHS の根 | CG 閉包 (近似、cg の回) | mod-2 閉包 | mod-k 閉包 | CG の下からの見積もり (3 つの最大) | CG の止まり方 | CG のラウンド・カット |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for p in PROBS:
    base = next((clo.get((p, m)) for m in MODES if clo.get((p, m)) and "lp" in clo.get((p, m))), None)
    if base is None:
        print(f"| {p} | (閉包なし) |||||||||")
        continue
    lp, s = base["lp"], base["sense"]
    z = opt.get(p)
    ro = roots.get((p, "ours"), {}).get("result", {})
    rh = roots.get((p, "highs"), {}).get("result", {})
    eb = ro.get("root_bound") if ro.get("root_bound") is not None else ro.get("bound")
    hb = rh.get("bound")
    vals = [pct(eb, lp, z, s), pct(hb, lp, z, s)]
    for m in MODES:
        c = clo.get((p, m))
        vals.append(pct(c["bound"] * s, lp, z, s) if c and "bound" in c else None)
    cl = [v for v in vals[2:] if v is not None]
    vals.append(max(cl) if cl else None)
    cg = clo.get((p, "cg"), {})
    last = (cg.get("hist") or [{}])[-1].get("sep", [{}])
    stop = cg.get("stop", cg.get("fail", ""))
    if stop == "nocut" and last:
        stop += "/" + str(last[0].get("st"))
    rows.append((p, vals))
    print(f"| {p} | {lp:.6g} | {z:.6g} | " + " | ".join(f(v) for v in vals) + f" | {stop} | {cg.get('rounds', '')}・{cg.get('ncut', '')} |")
json.dump(rows, open(D / "summ_rows.json", "w"))
