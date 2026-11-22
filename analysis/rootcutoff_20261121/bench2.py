"""7 節以降の比べ: python bench2.py root|full
root: 根だけ (ENOMOTO_MIP_NODE_LIMIT=1、1 スレッド、種 1、上限 300 秒)、full: 通し (4 スレッド、種 1〜3、上限 300 秒)。
設定は def・n (ENOMOTO_MIP_RSTLOOP=n)・k (ENOMOTO_MIP_TREE4_RCUNION=1 + ENOMOTO_MIP_RCSTRONG_KROOT=1)。結果は res.jsonl (tag = r2_CFG / f2_CFG)。記録済みは飛ばす。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
PROBS = ["swath1", "misc07", "neos-911970", "neos-1582420", "h80x6320d", "noswot", "roll3000", "p200x1188c", "exp-1-500-5-5", "10teams", "neos5", "mas76"]
CFGS = {"def": [], "n": ["ENOMOTO_MIP_RSTLOOP=n"], "k": ["ENOMOTO_MIP_TREE4_RCUNION=1", "ENOMOTO_MIP_RCSTRONG_KROOT=1"]}
if len(sys.argv) > 2: CFGS = {k: v for k, v in CFGS.items() if k in sys.argv[2].split(",")}
mode = sys.argv[1]
done = set()
if (D / "res.jsonl").exists():
    for l in open(D / "res.jsonl"):
        r = json.loads(l); done.add((r["problem"], r["seed"], r["tag"], r["threads"]))
jobs = [(p, 1, 1) for p in PROBS] if mode == "root" else [(p, s, 4) for s in (1, 2, 3) for p in PROBS]
pre = "r2_" if mode == "root" else "f2_"
extra = ["ENOMOTO_MIP_NODE_LIMIT=1"] if mode == "root" else []
for p, s, thr in jobs:
    for c, a in CFGS.items():
        if (p, s, pre + c, thr) in done: continue
        subprocess.run([sys.executable, str(D / "one.py"), p, str(s), pre + c, str(thr)] + a + extra, env={**os.environ, "RC_TL": "300"})
