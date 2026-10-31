"""58 問のうち時間内に解けうる 45 問 (bench5・bench6 の def・hs_pi0_bgw の平均が 58 秒未満) を 4 スレッド・60 秒で解き、bench.jsonl に足す。
使い方: python bench.py <設定名,...> <回> (設定は run.py の CFGS。設定の順は回ごとに逆にする)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from run import CFGS  # noqa: E402
PROBS = """gr4x6 flugpl gt2 beavma blend2 khb05250 sp150x300d p0201 fiber dcmulti markshare_4_0 p200x1188c drayage-100-23 qnet1 air03
nu25-pr12 exp-1-500-5-5 rout neos-827175 mik-250-20-75-4 neos8 binkar10_1 10teams drayage-25-23 mas76 misc07 nw04 neos17 neos-3083819-nubu
neos-911970 mc11 noswot pk1 hypothyroid-k1 ns1116954 swath1 neos-860300 eil33-2 h80x6320d qiu physiciansched6-2 roll3000 neos5 neos-960392
timtab1""".split()
OUT = D / "bench.jsonl"
if __name__ == "__main__":
    cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
    if rep % 2:
        cfgs = cfgs[::-1]
    done = set()
    if OUT.exists():
        for l in OUT.read_text().splitlines():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
    sys.path.insert(0, str(WT / "scripts"))
    import run_miplib_benchmark as rb  # noqa: E402
    for p in PROBS:
        rb.fetch(p, WT / ".miplib_cache")
        for c in cfgs:
            if (p, c, rep) in done:
                continue
            env = dict(os.environ, ENOMOTO_MIP_THREADS="4", **CFGS[c])
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(OUT, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
            print(p, c, rep, res.get("status"), res.get("time"), flush=True)
