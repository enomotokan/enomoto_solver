"""ENOMOTO_MIP_HNFPRE=count で、前処理の後の整数列だけの等式の HNF の解析の数を集める (2026-11-07)。
1 問ずつ worker を起こし、"HNFPRE: round 0" の行が出たら止める (出なければ時間切れまで待つ)。
使い方: python count.py out.jsonl [set ...] (set は hprobs.SETS の名前、既定は全部)"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
import hprobs
out = Path(sys.argv[1])
sets = sys.argv[2:] or list(hprobs.SETS)
done = set()
if out.exists():
    done = {json.loads(l)["problem"] for l in out.open()}
probs = list(dict.fromkeys(p for s in sets for p in hprobs.SETS[s]))
pat = re.compile(r"HNFPRE: round 0 eq (\d+) int_eq (\d+) rat_skip (\d+) comps (\d+) done (\d+) gave_up (\d+) cols (\d+) rank (\d+) fixes (\d+) congs (\d+) \(gen (\d+), tighten (\d+), max_mod (\d+)\) infeasible (\w+) \(([\d.]+)s\)")
keys = ["eq", "int_eq", "rat_skip", "comps", "done", "gave_up", "cols", "rank", "fixes", "congs", "congs_gen", "congs_tighten", "max_mod"]
for p in probs:
    if p in done:
        continue
    mps = WT / ".miplib_cache" / f"{p}.mps"
    if not mps.exists():
        print(p, "missing", flush=True)
        continue
    env = dict(os.environ, ENOMOTO_MIP_HNFPRE="count", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4")
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "4"],
                          stderr=subprocess.PIPE, stdout=subprocess.PIPE, env=env, text=True)
    rec = {"problem": p}
    presolve_line = None
    for line in pr.stderr:
        if line.startswith("Presolve:"):
            presolve_line = line.strip()
            break  # HNFPRE の行はこの行の前に出る
        m = pat.search(line)
        if m:
            rec.update({k: int(v) for k, v in zip(keys, m.groups()[:13])})
            rec["infeasible"] = m.group(14) == "true"
            rec["secs"] = float(m.group(15))
            break
    pr.kill()
    pr.wait()
    rec["presolve"] = presolve_line
    rec["wall"] = round(time.time() - t0, 1)
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec), flush=True)
