"""効く問題の確かめ (4 スレッド・60 秒、種 3・4): def・cd・cdc (連続分数だけ)・cdl (LLL だけ)。使い方: python queue_eff.py (eff.jsonl に足す、続きから)"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
OUT = "eff.jsonl"
PROBS = ["neos-3083819-nubu", "neos-911970", "hypothyroid-k1", "p200x1188c", "sp150x300d", "h80x6320d", "qnet1", "drayage-25-23"]
def done():
    s = set()
    if (D / OUT).exists():
        for l in open(D / OUT):
            try:
                r = json.loads(l); s.add((r["problem"], r["cfg"], r["rep"]))
            except Exception:
                pass
    return s
for rep in [3, 4]:
    for p in PROBS:
        for cfg in ["def", "cd", "cdc", "cdl"]:
            if (p, cfg, rep) in done():
                continue
            try:
                subprocess.run([sys.executable, str(D / "run.py"), cfg, p, "60", str(rep), OUT], timeout=60 + 600)
            except subprocess.TimeoutExpired:
                with open(D / OUT, "a") as f:
                    f.write(json.dumps({"problem": p, "cfg": cfg, "tl": 60, "rep": rep, "result": {"status": "killed"}}) + "\n")
