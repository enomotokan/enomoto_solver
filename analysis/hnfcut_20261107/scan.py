"""MIPLIB 2017 collection (scripts/miplib2017-v31.solu の 1065 問) から、HNFPRE の合同式が出て、根の最初の LP 点で合同式に合わない値の列がある
問題を探す (2026-11-07)。使い方: python scan.py out.jsonl [T=20] [最大の gz の MB=40]。
HNFPRE の記録で数えた問題 (58 問・Mittelmann・実行不能の 45 問・market split) は飛ばす。1 問ずつ: gz の大きさを HEAD で見て大きすぎれば飛ばし、
取ってきて ENOMOTO_MIP_HNFCUT=1 で T 秒解き、ログの "HNFPRE: round 0 ..."・"final table"・"HNFCUT: first LP point" を記録する。
合同式の出なかった問題の MPS は消す (ディスクを食わないように)。既に out にある問題は飛ばす。"""
import json, os, re, subprocess, sys, time, urllib.request
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/hnfpre_20261107"))
import run_miplib_benchmark as rb
import hprobs as hp
out = Path(sys.argv[1]); T = float(sys.argv[2]) if len(sys.argv) > 2 else 20.0; MAXMB = float(sys.argv[3]) if len(sys.argv) > 3 else 40.0
names = []
for l in (WT / "scripts/miplib2017-v31.solu").read_text().split("\n"):
    f = l.split()
    if len(f) >= 2 and f[1] not in names:
        names.append(f[1])
skip = set(hp.ALL)
done = set()
if out.exists():
    for l in out.open():
        try: done.add(json.loads(l)["problem"])
        except Exception: pass
cache = WT / ".miplib_cache"
F = r"([-+0-9.eE]+)"
for p in names:
    if p in skip or p in done:
        continue
    rec = {"problem": p}
    t0 = time.time()
    mps = cache / f"{p}.mps"
    had = mps.exists()
    try:
        if not had:
            req = urllib.request.Request(rb.BASE_URL + f"{p}.mps.gz", method="HEAD")
            with urllib.request.urlopen(req, timeout=60) as r:
                mb = int(r.headers.get("Content-Length", "0")) / 1e6
            rec["gz_mb"] = round(mb, 2)
            if mb > MAXMB:
                rec["skip"] = "big"
                raise StopIteration
        rb.fetch(p, cache)
        env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_HNFCUT="1")
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(T), "--threads", "4"],
                                capture_output=True, text=True, env=env, timeout=T * 4 + 120)
            se, so = pr.stderr, pr.stdout
        except subprocess.TimeoutExpired as e:
            se = e.stderr.decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); so = ""
            rec["timeout"] = True
        try: rec["result"] = {k: v for k, v in json.loads(so.strip().splitlines()[-1]).items() if k in ("status", "objective", "bound", "time", "nodes", "root_bound")}
        except Exception: pass
        m = re.search(r"HNFPRE: round 0 eq (\d+) int_eq (\d+) .*?cols (\d+) rank (\d+) fixes (\d+) congs (\d+) \(gen (\d+), tighten (\d+), max_mod (\d+)\) infeasible (\w+)", se)
        if m:
            rec["hnf"] = dict(zip(["eq", "int_eq", "cols", "rank", "fixes", "congs", "gen", "tighten", "max_mod"], map(int, m.groups()[:9])))
            rec["hnf"]["infeasible"] = m.group(10) == "true"
        m = re.search(r"HNFPRE: final table: (\d+) congruences", se)
        if m: rec["final_congs"] = int(m.group(1))
        m = re.search(r"HNFCUT: first LP point: cong cols (\d+) \(x frac (\d+), int but off (\d+), on (\d+) \(at bound (\d+)\)\)", se)
        if m: rec["lp"] = dict(zip(["cols", "frac", "off", "on", "atb"], map(int, m.groups())))
        m = re.search(r"MIP: root (HNFCUT stats: .*)", se)
        if m: rec["root_stats"] = m.group(1)[:400]
    except StopIteration:
        pass
    except Exception as e:
        rec["err"] = str(e)[:200]
    rec["wall"] = round(time.time() - t0, 1)
    keep = rec.get("final_congs", 0) > 0 or (rec.get("hnf") or {}).get("congs", 0) > 0 or (rec.get("hnf") or {}).get("infeasible")
    if not had and not keep and mps.exists():
        mps.unlink()
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, rec.get("skip", ""), rec.get("hnf", {}).get("congs"), rec.get("final_congs"), rec.get("lp"), rec["wall"], flush=True)
