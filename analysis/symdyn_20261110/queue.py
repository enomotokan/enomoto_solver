"""計測の列 (再開できる): python queue.py 列の名前。jobs の各行 (設定 問題 時間 回 出力 [root|highs]) のうち、出力にまだないものを順に回す"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
jobs = [l.split() for l in open(D / sys.argv[1]) if l.strip() and not l.startswith("#")]
for j in jobs:
    cfg, p, tl, rep, out = j[:5]
    kind = ""
    done = False
    if (D / out).exists():
        for l in open(D / out):
            try:
                r = json.loads(l)
            except Exception:
                continue
            if r["problem"] == p and r["cfg"] == cfg and str(r.get("rep", 0)) == rep and float(r["tl"]) == float(tl) :
                done = True
                break
    if done:
        continue
    if kind == "highs":
        subprocess.run([sys.executable, str(D / "highs_root.py"), p, tl, out])
    else:
        subprocess.run([sys.executable, str(D / "run.py"), cfg, p, tl, rep, out] + ([kind] if kind else []))
print("QUEUE DONE", sys.argv[1], flush=True)
