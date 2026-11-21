"""def と RCLOOP (r・fr) を、打ち切り値なしで 1 スレッド (決定的) と 4 スレッドで比べる: python bench.py
問題は 4 問 + 再スタートの起きる P の数問、種 1・2、上限 300 秒。one.py を呼び、結果は res.jsonl (tag = b_CFG)。記録済みは飛ばす。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
PROBS = ["swath1", "misc07", "neos-911970", "neos-1582420", "h80x6320d", "noswot", "roll3000", "p200x1188c", "exp-1-500-5-5", "10teams", "neos5", "mas76"]
CFGS = {"def": [], "r": ["ENOMOTO_MIP_RCLOOP=r"], "fr": ["ENOMOTO_MIP_RCLOOP=fr"]}
done = set()
if (D / "res.jsonl").exists():
    for l in open(D / "res.jsonl"):
        r = json.loads(l); done.add((r["problem"], r["seed"], r["tag"], r["threads"]))
for thr in (1, 4):
    for seed in (1, 2):
        for p in PROBS:
            for c, a in CFGS.items():
                if (p, seed, "b_" + c, thr) in done: continue
                subprocess.run([sys.executable, str(D / "one.py"), p, str(seed), "b_" + c, str(thr)] + a, env={**os.environ, "RC_TL": "300"})
