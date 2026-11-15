"""HiGHS (highs_s*.jsonl) と ENOMOTO の今の既定 (bench20 の設定 p、回 114) を Mittelmann 186 問で比べる (2026-11-03)。
時間切れ・60 秒超えは 60 秒。SGM は 10 秒ずらし。問題ごとの勝ち負けは (t+10) の比が 0.9 未満 / 1/0.9 超、両方時間切れは別に数える。"""
import glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
B = D.parent / "halve_submip_20261030"
def t_of(res, ok_status):
    t = res.get("time", 60.0) or 60.0
    return t if str(res.get("status", "")).lower() in ok_status and t <= 60 else 60.0
H = {}
for f in sorted(glob.glob(str(D / "highs_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l); H[r["problem"]] = t_of(r["result"], ("optimal", "infeasible"))
E = {"p": {}, "def": {}}
for f in glob.glob(str(B / "bench20_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["rep"] == 114 and r["cfg"] in E: E[r["cfg"]][r["problem"]] = t_of(r["result"], ("optimal", "infeasible"))
sgm = lambda ts: math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
for cfg in ("p", "def"):
    e = E[cfg]; ps = sorted(set(H) & set(e))
    print(f"=== ENOMOTO ({'今の既定 = bench20 の p' if cfg == 'p' else 'bench20 の def (p の前)'}) 対 HiGHS、{len(ps)} 問")
    hs = [p for p in ps if H[p] < 60]; es = [p for p in ps if e[p] < 60]
    print(f"解けた数: ENOMOTO {len(es)}、HiGHS {len(hs)}")
    print(f"SGM (186 問): ENOMOTO {sgm([e[p] for p in ps]):.2f} 秒、HiGHS {sgm([H[p] for p in ps]):.2f} 秒")
    both = [p for p in ps if H[p] < 60 or e[p] < 60]
    print(f"SGM (どちらかが解けた {len(both)} 問): ENOMOTO {sgm([e[p] for p in both]):.2f} 秒、HiGHS {sgm([H[p] for p in both]):.2f} 秒")
    w = l_ = tie = tl = 0
    rows = []
    for p in ps:
        if H[p] >= 60 and e[p] >= 60: tl += 1; continue
        r = (e[p] + 10) / (H[p] + 10)
        if r < 0.9: w += 1; tag = "E 勝"
        elif r > 1 / 0.9: l_ += 1; tag = "H 勝"
        else: tie += 1; tag = "互角"
        rows.append((r, p, H[p], e[p], tag))
    print(f"問題ごと: ENOMOTO の勝ち {w}、HiGHS の勝ち {l_}、互角 {tie}、両方時間切れ {tl}")
    print("HiGHS だけが解けた:", ", ".join(f"{p} ({H[p]:.1f})" for p in ps if H[p] < 60 <= e[p]) or "なし")
    print("ENOMOTO だけが解けた:", ", ".join(f"{p} ({e[p]:.1f})" for p in ps if e[p] < 60 <= H[p]) or "なし")
    if cfg == "p":
        print("両方解けた問題の勝ち負け (比 = (E+10)/(H+10)):")
        for r, p, h, ee, tag in sorted(rows):
            if h < 60 and ee < 60: print(f"  {tag} {p:28s} HiGHS {h:5.1f}  ENOMOTO {ee:5.1f}  比 {r:.2f}")
    print()
