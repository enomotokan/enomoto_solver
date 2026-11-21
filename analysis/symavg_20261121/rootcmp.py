"""根だけ (1 スレッド・ノード上限 1・600 秒・種 1) で、既定と SYMAVG (=1・=v) と既存の SYM_CUTS (像をプールに) を比べる。
python rootcmp.py → rootcmp.jsonl (途中から再開できる、3 本並べる)。対象は対称性が見つかり根で対称性を崩す処理のない問題 (README 1 節)。"""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = "misc07 neos-911970 noswot mas74 mas76 rout nu25-pr12 fastxgemm-n2r6s0t2 mcsched sct2 wachplan ns1208400 uccase12 lectsched-5-obj chromaticindex512-7 chromaticindex1024-7 neos-2746589-doon neos-2978193-inde".split()
CFG = {"def": {}, "avg1": {"ENOMOTO_MIP_SYMAVG": "1"}, "avgv": {"ENOMOTO_MIP_SYMAVG": "v"}, "symcuts": {"ENOMOTO_MIP_SYM_CUTS": "1"}}
OUT = D / "rootcmp.jsonl"
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in OUT.read_text().splitlines()} if OUT.exists() else set()
def one(pc):
    p, c = pc
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", **CFG[c]})
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "1"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=900)
    (D / "logs" / f"{p}_{c}.log").write_text("\n".join(l for l in pr.stderr.splitlines() if "MIP: cut" in l or "symmetry" in l or "SYMAVG" in l or "root LP" in l))
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    g = lambda pat: (re.findall(pat, pr.stderr) or [None])[-1]  # noqa: E731
    return {"problem": p, "cfg": c, "result": res, "cutloop": g(r"cut loop done: (.*)"), "symavg": g(r"SYMAVG: (.*)"), "images": len(re.findall(r"symmetric images", pr.stderr))}
jobs = [(p, c) for p in PROBS for c in CFG if (p, c) not in done]
with ThreadPoolExecutor(3) as ex:
    for row in ex.map(one, jobs):
        with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
        print(row["problem"], row["cfg"], row["result"].get("root_bound"), row["result"].get("time"), row["symavg"], flush=True)
