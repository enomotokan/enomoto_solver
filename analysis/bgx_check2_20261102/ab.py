"""2. 比べ: 洗い出しで `passed` の問題を、既定 (def) と `ENOMOTO_MIP_BG_CUTS_BASIS_X=0` (x0) で交互に解く (4 スレッド・60 秒、ENOMOTO_MIP_LOG=1、
ソルバーは同時に 1 つ)。引数: 問題 (カンマ区切り)・回数。出力 ab.jsonl、ログ logs/{問題}_{形}_{回}.log。
写しのラウンド (`rounds so far`)、終わりの `N rounds, obj ...`・使われたか、写しの根の LP (`basis nb diag` の bgcopy)、根の下界などを抜き出す。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "x0": {"ENOMOTO_MIP_BG_CUTS_BASIS_X": "0"}}
out = D / "ab.jsonl"


def parse(log: str) -> dict:
    g = {}
    m = re.search(r"background cuts: basis with the column values (passed|not passed) \(root LP ([0-9.]+)s\)", log)
    g["bx"] = m.group(1) if m else None
    g["root_lp"] = float(m.group(2)) if m else None
    m = re.search(r"cut loop done: .*obj ([-0-9.e+]+) -> ([-0-9.e+]+)", log)
    g["cutloop_obj"] = float(m.group(2)) if m else None
    m = re.search(r"background cuts: rounds so far \[(.*)\]", log)
    g["so_far"] = m.group(1) if m else None
    m = re.search(r"background cuts: waited ([0-9.]+)s for progress \(cap ([0-9.]+)s, ([^)]*)\), (\d+) rounds, obj (\S+) \(([0-9.]+)s\)", log)
    if m:
        g["wait"] = float(m.group(1)); g["wait_stop"] = m.group(3); g["stop_at"] = float(m.group(6))
    m = re.search(r"MIP: background cuts: (\d+) rounds, (.*)", log)
    g["bg_end"] = m.group(0)[len("MIP: background cuts: "):] if m else None
    g["bg_used"] = None if not m else ("not used" not in m.group(2))
    m = re.search(r"bgcopy (\d+) lps (\d+) iters \([0-9.]+/LP\) ([0-9.]+)s", log)
    g["bgcopy"] = f"{m.group(2)} it {m.group(3)}s" if m else None
    m = re.search(r"main 1 lps (\d+) iters \([0-9.]+/LP\) ([0-9.]+)s", log)
    g["main_lp"] = f"{m.group(1)} it {m.group(2)}s" if m else None
    m = re.search(r"after root heuristics: incumbent (.*) \(([0-9.]+)s\)", log)
    g["heur_end"] = float(m.group(2)) if m else None
    return g


if __name__ == "__main__":
    probs, reps = sys.argv[1].split(","), int(sys.argv[2])
    start = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    for p in probs:
        mps = rb.fetch(p, WT / ".miplib_cache")
        for k in range(start, start + reps):
            for c, e in (CFGS.items() if k % 2 == 0 else reversed(list(CFGS.items()))):
                env = {k2: v for k2, v in os.environ.items() if not k2.startswith("ENOMOTO_")}
                env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **e)
                r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                   cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                log = r.stderr + r.stdout
                (D / "logs" / f"{p}_{c}_{k}.log").write_text(log)
                st = [l for l in r.stdout.splitlines() if l.startswith("{")]
                res = json.loads(st[-1]) if st else {"status": "error"}
                rec = {"prob": p, "cfg": c, "rep": k, **{x: res.get(x) for x in ("status", "time", "objective", "bound", "root_bound", "nodes")}, **parse(log)}
                with out.open("a") as f:
                    f.write(json.dumps(rec) + "\n")
                print(json.dumps(rec), flush=True)
