"""計画の行 (`e 設定 問題 秒 種` = ENOMOTO run1.py、`h 設定 問題 秒` = HiGHS hrun.py) を上から順に 1 つずつ回す。
出力 (e.jsonl / h.jsonl) に既にある組は飛ばすので、止まっても同じ命令で続きから回る。
使い方: python drive.py 計画.txt"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
def done(fn, key):
    p = D / fn
    if not p.exists():
        return set()
    s = set()
    for l in p.read_text().splitlines():
        try:
            r = json.loads(l)
        except Exception:
            continue
        s.add(key(r))
    return s
for line in Path(sys.argv[1]).read_text().splitlines():
    a = line.split()
    if not a or a[0].startswith("#"):
        continue
    if a[0] == "e":
        _, cfg, p, tl, seed = a[:5]
        out = a[5] if len(a) > 5 else "e.jsonl"
        if (p, cfg, int(tl), int(seed)) in done(out, lambda r: (r["problem"], r["cfg"], r["tl"], r["rep"])):
            continue
        subprocess.run([sys.executable, str(D / "run1.py"), str(D / out), cfg, p, tl, seed])
    elif a[0] == "h":
        _, cfg, p, tl = a[:4]
        out = a[4] if len(a) > 4 else "h.jsonl"
        if (p, cfg, float(tl)) in done(out, lambda r: (r["problem"], r["cfg"], r["tl"])):
            continue
        subprocess.run([sys.executable, str(D / "hrun.py"), str(D / out), cfg, p, tl])
