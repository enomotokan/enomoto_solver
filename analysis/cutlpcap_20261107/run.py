"""1 回の計測 (2026-11-07、analysis/cutlpcap_20261107.md、modkvb_20261107/run.py を写したもの): ENOMOTO を解き、結果と根の情報
(カットのループごとの本数・LP の値・時刻) を jsonl に足し、詳細ログを logs/<設定>_<問題>_<時間>_r<回>[_root].err.gz に置く。
ENOMOTO_ の環境変数は消してから設定のものだけを与える。
使い方: python run.py 設定 問題 時間 回 出力 [root]  (設定は CFGS のキー。root ならノード上限 1 で根だけ。回は ENOMOTO_T_MIP_SEED にも使う)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
L = {"ENOMOTO_MIP_CUTLPCAP_LOG": "1"}
CFGS = {
    "def": dict(L),
    "a": {"ENOMOTO_MIP_CUTLPCAP": "al"},
    "n": {"ENOMOTO_MIP_CUTLPCAP": "nl"},
    "c": {"ENOMOTO_MIP_CUTLPCAP": "cl"},
    "an": {"ENOMOTO_MIP_CUTLPCAP": "anl"},
    "ac": {"ENOMOTO_MIP_CUTLPCAP": "acl"},
    # MODKVB (並行の試作) と合わせたもの
    "mk": {"ENOMOTO_MIP_MODKVB": "1", **L},
    "mka": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_CUTLPCAP": "al"},
    "mkn": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_CUTLPCAP": "nl"},
    "mkc": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_CUTLPCAP": "cl"},
    # HNFSUBST (並行の試作、analysis/hnfsubst_20261107.md) と合わせたもの (mzzv11 が同じ型と知らせを受けた)
    "hs": {"ENOMOTO_MIP_HNFSUBST": "1", **L},
    "hsa": {"ENOMOTO_MIP_HNFSUBST": "1", "ENOMOTO_MIP_CUTLPCAP": "al"},
}
cfg, p, tl, rep, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
root = len(sys.argv) > 6 and sys.argv[6] == "root"
OUT = D / out
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[cfg])
if root:
    env["ENOMOTO_MIP_NODE_LIMIT"] = "1"
t0 = time.time()
try:
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    err = r.stderr
except Exception as e:  # noqa: BLE001
    res = {"status": "crash", "error": str(e)[:300]}
    err = getattr(e, "stderr", "") or ""
    if isinstance(err, bytes):
        err = err.decode(errors="replace")
loops, last_t, rounds = [], None, 0
for l in err.splitlines():
    m = re.search(r"MIP: cut round (\d+): .*\(([\d.]+)s\)$", l)
    if m:
        last_t = float(m.group(2)); rounds = int(m.group(1)) + 1
    m = re.search(r"MIP: cut loop done: (\d+) cuts added, (\d+) in LP.*obj (\S+) -> (\S+)", l)
    if m:
        loops.append({"added": int(m.group(1)), "in_lp": int(m.group(2)), "obj0": float(m.group(3)), "obj1": float(m.group(4)), "t": last_t, "rounds": rounds})
        last_t, rounds = None, 0
sep = [l for l in err.splitlines() if l.startswith("SEP modkvb")]
mk = {"calls": len(sep), "cands": 0, "time": 0.0, "timeout": 0}
for l in sep:
    m = re.search(r"cands (\d+)", l); mk["cands"] += int(m.group(1)) if m else 0
    m = re.search(r" ([\d.]+)s ", l); mk["time"] += float(m.group(1)) if m else 0
    mk["timeout"] += " timeout 1 " in l
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"{cfg}_{p}_{tl}_r{rep}{'_root' if root else ''}.err.gz", "wt") as f:
    f.write(err)
sys.path.insert(0, str(D))
import parse as _pz  # noqa: E402
pz = _pz.parse(err); pz["rounds"] = {str(k): v for k, v in pz["rounds"].items()}
rec = {"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "root": root, "wall": round(time.time() - t0, 1), "result": res, "root_loops": loops[:4],
       "sep": mk, "sep_first": sep[:2], "warn": err.count("MIP: WARNING"), "cutloop": pz, "cl_a": err.count("CUTLPCAP a:"), "cl_n": err.count("CUTLPCAP n:"), "at": time.strftime("%H:%M:%S")}
with open(OUT, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), loops[:1], mk, flush=True)
