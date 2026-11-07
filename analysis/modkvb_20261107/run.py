"""1 回の計測 (2026-11-07、analysis/modkvb_20261107.md、mushroom_cuts_20261106/run.py を写したもの): ENOMOTO を解き、結果と根の情報
(カットのループごとの本数・LP の値・時刻) を jsonl に足し、詳細ログを logs/<設定>_<問題>_<時間>_r<回>[_root].err.gz に置く。
ENOMOTO_ の環境変数は消してから設定のものだけを与える。
使い方: python run.py 設定 問題 時間 回 出力 [root]  (設定は CFGS のキー。root ならノード上限 1 で根だけ。回は ENOMOTO_T_MIP_SEED にも使う)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "mc": {"ENOMOTO_MIP_MUSHAGG": "cover", "ENOMOTO_MIP_MUSHAGG_LOG": "1"},
    "mk": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1"},
    "mkc": {"ENOMOTO_MIP_MODKVB": "cover", "ENOMOTO_MIP_MODKVB_LOG": "1"},
    "mkg": {"ENOMOTO_MIP_MODKVB": "gen", "ENOMOTO_MIP_MODKVB_LOG": "1"},
    # 候補の数の上限つき (wheel は /tmp/pyw4、上限の入ったもの)
    "mk50": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1", "ENOMOTO_T_MIP_MODKVB_MAXCANDS": "50", "PYTHONPATH": "/tmp/pyw4"},
    # 5 章 (wheel は /tmp/pyw5 = 35ec5a71 + 候補の上限 + 診断の口、pyw5_from_35ec5a71.patch)
    "def5": {"PYTHONPATH": "/tmp/pyw5"},
    "nozh": {"ENOMOTO_MIP_NO_ZEROHALF": "1", "PYTHONPATH": "/tmp/pyw5"},
    "mk50b": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1", "ENOMOTO_T_MIP_MODKVB_MAXCANDS": "50", "PYTHONPATH": "/tmp/pyw5"},
    "mk50nozh": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1", "ENOMOTO_T_MIP_MODKVB_MAXCANDS": "50", "ENOMOTO_MIP_NO_ZEROHALF": "1", "PYTHONPATH": "/tmp/pyw5"},
    "mk50novb": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1", "ENOMOTO_T_MIP_MODKVB_MAXCANDS": "50", "ENOMOTO_MIP_MODKVB_NOVB": "1", "PYTHONPATH": "/tmp/pyw5"},
    "mk50nocut": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1", "ENOMOTO_T_MIP_MODKVB_MAXCANDS": "50", "ENOMOTO_MIP_MODKVB_NOCUTROWS": "1", "PYTHONPATH": "/tmp/pyw5"},
    "mk50k2": {"ENOMOTO_MIP_MODKVB": "1", "ENOMOTO_MIP_MODKVB_LOG": "1", "ENOMOTO_T_MIP_MODKVB_MAXCANDS": "50", "ENOMOTO_MIP_MODKVB_K2": "1", "PYTHONPATH": "/tmp/pyw5"},
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
sep = [l for l in err.splitlines() if l.startswith("SEP modkvb") or l.startswith("SEP mushagg")]
mk = {"calls": len(sep), "cands": 0, "time": 0.0, "timeout": 0}
for l in sep:
    m = re.search(r"cands (\d+)", l); mk["cands"] += int(m.group(1)) if m else 0
    m = re.search(r" ([\d.]+)s ", l); mk["time"] += float(m.group(1)) if m else 0
    mk["timeout"] += " timeout 1 " in l
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"{cfg}_{p}_{tl}_r{rep}{'_root' if root else ''}.err.gz", "wt") as f:
    f.write(err)
rec = {"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "root": root, "wall": round(time.time() - t0, 1), "result": res, "root_loops": loops[:4],
       "sep": mk, "sep_first": sep[:2], "warn": err.count("MIP: WARNING"), "at": time.strftime("%H:%M:%S")}
with open(OUT, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), loops[:1], mk, flush=True)
