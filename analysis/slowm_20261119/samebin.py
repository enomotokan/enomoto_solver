"""same/bin の形 (行の他の係数がすべて同符号の 0-1 列) が害かの確かめ: 1 問を {def, 1g} × 種で 4 スレッドで解き、ログから前処理・根・木の数を抜く。
使い方: python samebin.py 問題 秒 種の数 [形,形] → logs/sb/問題_設定_s種.log、samebin_問題.jsonl"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
P, TL, NS = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
# 4 つ目の引数で INTSING の形を並べる (既定 1g)。例: python samebin.py air03 60 10 1g,1go
CF = {"def": {}} | {v: {"ENOMOTO_MIP_INTSING": v} for v in (sys.argv[4] if len(sys.argv) > 4 else "1g").split(",")}
PAT = {"pre_rows": r"presolve: rows \d+ -> (\d+), free columns \d+ -> (\d+), nonzeros \d+ -> (\d+)",
       "domfix": r"(\d+) dominated binaries fixed", "rootlp": r"MIP: root LP ([-\d.e+]+) \(", "cutobj": r"cut loop done: .*obj [-\d.e+]+ -> ([-\d.e+]+)",
       "root_end": r"root LP bound after cuts and heuristics ([-\d.e+]+)", "gfix": r"global fixed integers (\d+) of",
       "nodes": r"MIP: +([\d.]+)s nodes +(\d+) ", "sbl": r"branching: (\d+) selections"}
out = open(D / f"samebin_{P}.jsonl", "a")
for s in range(1, NS + 1):
    for c, e in CF.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(e, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(s))
        p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{P}.mps"), "--time-limit", str(TL), "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 600)
        (D / "logs/sb" / f"{P}_{c}_s{s}.log").write_text(p.stderr)
        res = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
        rec = {"problem": P, "cfg": c, "seed": s, "result": res[-1] if res else None}
        for k, pat in PAT.items():
            m = re.findall(pat, p.stderr)
            rec[k] = m if k == "pre_rows" else (m[-1] if m else None)
        out.write(json.dumps(rec) + "\n"); out.flush()
        r = rec["result"] or {}
        print(s, c, r.get("status"), r.get("time"), rec["nodes"], rec["rootlp"], rec["cutobj"], rec["domfix"], rec["gfix"], rec["pre_rows"][-1:] , flush=True)
