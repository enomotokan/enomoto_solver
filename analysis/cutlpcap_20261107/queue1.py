"""待ち行列 1 (続きから回せる): Mittelmann の既定の根 (modkvb_20261107 にログのない 45 問、90 秒、ノード上限 1) -> rootmitt_def.jsonl
その後、上限に当たった問題の HiGHS の根 (highs_root.jsonl)"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
def done(out):
    s = set()
    if (D / out).exists():
        for l in open(D / out):
            try:
                r = json.loads(l); s.add((r["problem"], r["cfg"], r.get("rep", 1)))
            except Exception:
                pass
    return s
def run(p, cfg, tl, out, rep=1, root=True):
    if (p, cfg, rep) in done(out):
        return
    if cfg == "highs":
        cmd = [sys.executable, str(D / "highs_root.py"), p, str(tl), out]
    else:
        cmd = [sys.executable, str(D / "run.py"), cfg, p, str(tl), str(rep), out] + (["root"] if root else [])
    try:
        subprocess.run(cmd, timeout=tl + 600)
    except subprocess.TimeoutExpired:
        with open(D / out, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "root": root, "result": {"status": "killed"}}) + "\n")
if __name__ == "__main__":
    for p in json.load(open(D / "mitt_missing.json")):
        run(p, "def", 90, "rootmitt_def.jsonl")
    for p in json.load(open(D / "highs_todo.json")) if (D / "highs_todo.json").exists() else []:
        run(p, "highs", 90, "highs_root.jsonl")
