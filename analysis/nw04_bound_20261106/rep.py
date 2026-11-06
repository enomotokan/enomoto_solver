"""nw04 を設定・種ごとに 4 スレッド・60 秒で解き、ログを logs/ に、結果を rep.jsonl に足す (2026-11-06、nw04_bound)。
使い方: python rep.py <問題> <設定,...> <種の始め> <種の終わり>
各回について、再スタートの前と後で根の LP・カットのループ・裏のカットの目的値の最大を拾い、最適値 (OPT) を越えたかを記す。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "h1mv": {"ENOMOTO_MIP_CUT_HARRIS2": "1", "ENOMOTO_MIP_HO4_CUT0": "moved"},
    "h1": {"ENOMOTO_MIP_CUT_HARRIS2": "1"},
    "sh1": {"ENOMOTO_SIMPLEX_HARRIS2": "1"},
    "shr": {"ENOMOTO_SIMPLEX_HARRIS2": "r"},
    "h1mv_noclamp": {"ENOMOTO_MIP_CUT_HARRIS2": "1", "ENOMOTO_MIP_HO4_CUT0": "moved", "ENOMOTO_MIP_ROOT_BOUND_CLAMP": "0"},
}
OUTF = os.environ.get("REP_OUT", "rep.jsonl")  # 直した後の計測は REP_OUT=rep_fix.jsonl
OPT = {"nw04": 16862.0}
NUM = r"([-+0-9.eE]+)"
def scan(log):
    """再スタートの区切りごとに、記録された根の LP 系の値の最大を返す"""
    segs = [[]]
    for l in log.splitlines():
        if re.match(r"MIP: restart \d+ \(", l):
            segs.append([])
        for pat in (r"MIP: root LP " + NUM + r" \(", r"MIP: cut round \d+: .* obj " + NUM + r" \(", r"MIP: cut loop done: .* -> " + NUM,
                    r"MIP: background cuts: .* obj " + NUM + r" -> " + NUM, r"Root cuts: objective .* -> " + NUM):
            m = re.search(pat, l)
            if m:
                try:
                    segs[-1].append(float(m.groups()[-1]))
                except ValueError:
                    pass
    return [max(s) if s else None for s in segs]
if __name__ == "__main__":
    prob, cfgs, s0, s1 = sys.argv[1], sys.argv[2].split(","), int(sys.argv[3]), int(sys.argv[4])
    mps = rb.fetch(prob, WT / ".miplib_cache")
    for seed in range(s0, s1 + 1):
        for c in cfgs:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(seed), **CFGS[c])
            t = time.time()
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            (D / "logs" / f"{prob}_{c}_s{seed}{'_fix' if OUTF != 'rep.jsonl' else ''}.log").write_text(r.stderr)
            try:
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:200]}
            seg = scan(r.stderr)
            opt = OPT.get(prob)
            over = opt is not None and any(v is not None and v > opt + 1e-6 for v in [res.get("root_bound"), res.get("bound")])
            pre_over = opt is not None and seg[0] is not None and seg[0] > opt + 1e-6
            rec = {"problem": prob, "cfg": c, "seed": seed, "env": CFGS[c], "result": res, "seg_max": seg, "restarts": len(seg) - 1,
                   "over": over, "pre_restart_over": pre_over, "wall": round(time.time() - t, 1), "at": time.strftime("%H:%M:%S")}
            with open(D / OUTF, "a") as f:
                f.write(json.dumps(rec) + "\n")
            print(prob, c, seed, res.get("status"), res.get("objective"), res.get("root_bound"), seg, over, pre_over, flush=True)
