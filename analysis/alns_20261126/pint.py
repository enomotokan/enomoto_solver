"""上界の弱い問題で、暫定解の時間ごとの推移 (primal integral) を比べる: python pint.py スレッド数 並べる数 [設定 ...]。
問題 PROBS × 設定 (cfg.py) × 種 SEEDS を時間の上限 TL 秒で解き (ENOMOTO_MIP_LOG=1)、`MIP: new incumbent Z at node N (T s) [出どころ]` の行から
暫定解の推移を取り、pint_t<スレッド>.jsonl に足す (済んだ組は飛ばす。近傍の状態をスレッドごとに持った初めの版の結果は pint_v1_t*.jsonl)。
primal integral (Berthold): 主のギャップ γ(t) = |z(t) − z*| / max(|z(t)|, |z*|) (解がない間は 1、z* は miplib2017-v31.solu の値) を
[0, TL] で積分し、TL で割ったもの (0〜1、小さいほど良い)。ログは logs/ (コミットしない)。"""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
PROBS = os.environ.get("ALNS_PROBS", "neos-1456979 comp07-2idx comp21-2idx map10 momentum1 neos-4532248-waihi neos-5107597-kakapo satellites2-40 trento1 lectsched-5-obj").split()
SEEDS = [int(s) for s in os.environ.get("ALNS_SEEDS", "1").split()]
TL = float(os.environ.get("ALNS_TL", "120"))
REF = {}
for l in open(R / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="):
        REF[t[1]] = float(t[2])
PAT = re.compile(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]")
def pint(traj, ref, tl):
    if ref is None:
        return None
    g = lambda z: 0.0 if abs(z - ref) <= 1e-9 * max(1.0, abs(ref)) else (1.0 if z * ref < 0 else abs(z - ref) / max(abs(z), abs(ref)))
    area, t_prev, gam = 0.0, 0.0, 1.0
    for t, z, _ in traj:
        t = min(t, tl)
        area += gam * (t - t_prev)
        t_prev, gam = t, g(z)
    area += gam * (tl - t_prev)
    return area / tl
def one(job):
    th, p, c, s = job
    log = D / "logs" / f"t{th}_{c}_{p}_s{s}.log"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[c]); env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(s)})
    with open(log, "w") as f:
        r = subprocess.run([sys.executable, str(R / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(R / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(TL), "--threads", str(th)],
                           cwd=R, env=env, stdout=subprocess.PIPE, stderr=f, text=True, timeout=TL + 600)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    traj = []
    for l in open(log, errors="replace"):
        m = PAT.search(l)
        if m:
            traj.append((float(m.group(3)), float(m.group(1)), m.group(4)))
    row = {"problem": p, "cfg": c, "seed": s, "threads": th, "tl": TL, "result": res, "traj": traj, "pint": pint(traj, REF.get(p), TL),
           "alns_cp": sum(1 for l in open(log, errors="replace") if "ALNS-CP" in l)}
    with open(D / f"pint_t{th}.jsonl", "a") as f:
        f.write(json.dumps(row) + "\n")
    print(th, c, p, s, res["status"], res.get("objective"), round(row["pint"] or -1, 4), row["alns_cp"], flush=True)
if __name__ == "__main__":
    th = int(sys.argv[1]); nj = int(sys.argv[2]) if len(sys.argv) > 2 else 1
    cf = sys.argv[3:] or list(CFGS)
    (D / "logs").mkdir(exist_ok=True)
    out = D / f"pint_t{th}.jsonl"
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"], json.loads(l)["seed"]) for l in open(out)} if out.exists() else set()
    jobs = [(th, p, c, s) for s in SEEDS for p in PROBS for c in cf if (p, c, s) not in done]
    with ThreadPoolExecutor(nj) as ex:
        list(ex.map(one, jobs))
