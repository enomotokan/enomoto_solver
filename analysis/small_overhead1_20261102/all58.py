"""58 問 (analysis/cbcimpl_20261029/probs.py) に ENOMOTO_MIP_SMALL1=wr を 1 回ずつかけ、試作が働いた (ログに
"not waiting (ENOMOTO_MIP_SMALL1" の行がある) 問題だけ既定でも 1 回解く (働かない問題は既定と同じ経路)。結果は all58.jsonl。"""
import json, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
OUT = D / "all58.jsonl"
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
def run(p, cfg, extra):
    if (p, cfg) in done: return None
    subprocess.run([sys.executable, str(D / "run.py"), p, f"a58_{cfg}", "1", *extra], cwd=D, capture_output=True, text=True, timeout=400)
    rec = [json.loads(l) for l in (D / "results.jsonl").read_text().splitlines()]
    r = [x for x in rec if x["problem"] == p and x["cfg"] == f"a58_{cfg}"][-1]
    log = (D / "logs" / f"{p}_a58_{cfg}_0.log").read_text()
    act = "not waiting (ENOMOTO_MIP_SMALL1" in log
    nrst = log.count("MIP: restart ") - log.count("MIP: restart check")
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "active": act, "restart_lines": nrst, "result": r["result"], "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, act, r["result"].get("status"), round(r["result"].get("time", 0), 2), flush=True)
    return act
for p in PROBS:
    a = run(p, "s1wr", ["ENOMOTO_MIP_SMALL1=wr"])
    if a is None:
        a = any(json.loads(l)["active"] for l in OUT.read_text().splitlines() if json.loads(l)["problem"] == p and json.loads(l)["cfg"] == "s1wr")
    if a: run(p, "base", [])
