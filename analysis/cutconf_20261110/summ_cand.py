"""効きそうな問題での既定と試作の比べ (cand*.jsonl)。使い方: python summ_cand.py ファイル.jsonl [設定 ...]
問題・種ごとに、時間 (解けなければ時間上限)・下界・暫定解・ノード数と cutconf の統計の要点を並べる。
最適値 (scripts/miplib2017-v31.solu) と矛盾する暫定解・下界 (最小化で 暫定解 < 最適値、下界 > 最適値) には「矛盾」と書く。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
SOLU = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    p = l.split()
    if len(p) >= 3 and p[0] in ("=opt=", "=best="):
        SOLU[p[1]] = (p[0], float(p[2]))
R = defaultdict(dict)
for l in open(sys.argv[1]):
    if l.strip():
        r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r
cfgs = sys.argv[2:] or sorted({c for v in R.values() for c in v})


def conflict(p, res):
    if p not in SOLU or SOLU[p][0] != "=opt=":
        return ""
    o = SOLU[p][1]; tol = 1e-6 * max(1.0, abs(o))
    ob, bd = res.get("objective"), res.get("bound")
    bad = []
    if ob is not None and ob < o - tol:
        bad.append("暫定解<最適")
    if bd is not None and bd > o + tol:
        bad.append("下界>最適")
    return "矛盾(" + ",".join(bad) + ")" if bad else ""


def cc_key(s):
    m = re.search(r"out (\d+)", s); u = re.search(r"pruned (\d+) tightened (\d+)", s)
    return (m.group(1) if m else "-") + (f" 使用 刈{u.group(1)}/締{u.group(2)}" if u else "")


print("| 問題 | 種 | " + " | ".join(f"{c} 時間 | {c} 下界 / 暫定解 | {c} ノード" for c in cfgs) + " | cutconf 制約 |")
print("|" + "---|" * (2 + 3 * len(cfgs) + 1))
for (p, s) in sorted(R):
    row = [p, str(s)]
    ccs = ""
    for c in cfgs:
        r = R[(p, s)].get(c)
        if not r:
            row += ["-", "-", "-"]; continue
        res = r["result"]
        t = res.get("time") if res.get("status") == "optimal" else None
        row += [f"{t:.1f}" if t is not None else res.get("status", "?"),
                f"{res.get('bound', float('nan')):.6g} / {res.get('objective') if res.get('objective') is None else format(res.get('objective'), '.6g')} {conflict(p, res)}",
                str(res.get("nodes"))]
        if r.get("cc"):
            ccs += f"{c}: {cc_key(r['cc'])} "
    print("| " + " | ".join(row) + f" | {ccs} |")
