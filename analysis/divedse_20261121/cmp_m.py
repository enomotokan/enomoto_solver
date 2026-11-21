"""run_m.py の結果 (res_m_*.jsonl) の def と dc の比べ: python cmp_m.py
問題・種ごとに状態・時間・下界・解・根の潜りの秒・作り直しを省いた回数を並べ、解けた数と時間の比 (両方解けたもの) をまとめる。"""
import glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
for fn in sorted(glob.glob(str(D / "res_m_*.jsonl"))):
    for l in open(fn):
        r = json.loads(l); rows[(r["problem"], r["seed"], r["cfg"])] = r
f = lambda v: "-" if v is None else f"{v:.8g}"
print(f"{'問題':22s} 種 {'設定':4s} {'状態':>10s} {'秒':>7s} {'下界':>14s} {'解':>14s} {'節':>7s} {'潜り秒':>7s} skip/built/noest")
solved = {"def": 0, "dc": 0}; rt = []; win = {"def": 0, "dc": 0}
for (p, s, c) in sorted(rows):
    r = rows[(p, s, c)]; res = r["result"]
    if res.get("status") == "optimal": solved[c] += 1
    print(f"{p:22s} {s}  {c:4s} {str(res.get('status')):>10s} {res.get('time') or 0:7.1f} {f(res.get('bound')):>14s} {f(res.get('objective')):>14s} {res.get('nodes') or 0:>7} {sum(d[2] for d in r['dives']):7.1f} {r['dse_cost']}")
    if c == "dc" and (p, s, "def") in rows:
        a = rows[(p, s, "def")]["result"]
        if a.get("status") == "optimal" and res.get("status") == "optimal": rt.append(math.log(res["time"] / a["time"]))
        elif a.get("objective") is not None and res.get("objective") is not None and a["objective"] != res["objective"]:
            win["dc" if res["objective"] < a["objective"] else "def"] += 1  # 最小化の問題として比べる
g = math.exp(sum(rt) / len(rt)) if rt else float("nan")
print(f"解けた数 def {solved['def']} dc {solved['dc']}; 両方解けた {len(rt)} 本の時間の比の幾何平均 {g:.3f}; 解けない回の暫定解の良いほう {win}")
