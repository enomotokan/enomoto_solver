"""1 回の計測 (2026-11-06、analysis/mushroom_cuts_20261106.md、intvub_20261106/run.py を写したもの): ENOMOTO を 4 スレッドで解き、結果と根の情報 (カットのループごとの本数・LP の値・時刻)
を res.jsonl に足し、詳細ログを logs/<設定>_<問題>_<時間>_r<回>.err.gz に置く。ENOMOTO_ の環境変数は消してから設定のものだけを与える。
使い方: python run.py 設定 問題 時間 回 [出力の名前]  (設定は CFGS のキー)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "mc": {"ENOMOTO_MIP_MUSHAGG": "cover", "ENOMOTO_MIP_MUSHAGG_LOG": "1"},
    "m1": {"ENOMOTO_MIP_MUSHAGG": "1", "ENOMOTO_MIP_MUSHAGG_LOG": "1"},
    "iv1": {"ENOMOTO_MIP_INTVUB": "1"},
    "iv2": {"ENOMOTO_MIP_INTVUB": "2"},
    "iv3": {"ENOMOTO_MIP_INTVUB": "3"},
    "iv1i": {"ENOMOTO_MIP_INTVUB": "1", "ENOMOTO_MIP_INTVUB_IMPL": "1"},
    "iv3i": {"ENOMOTO_MIP_INTVUB": "3", "ENOMOTO_MIP_INTVUB_IMPL": "1"},
}
cfg, p, tl, rep = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
OUT = D / (sys.argv[5] if len(sys.argv) > 5 else "res.jsonl")
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_INTVUB_LOG="1", **CFGS[cfg])
t0 = time.time()
try:
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    err = r.stderr
except Exception as e:  # noqa: BLE001
    res = {"status": "crash", "error": str(e)[:300]}
    err = ""
# 根のカットのループ: 「cut loop done: N cuts added, ... obj a -> b」と、その直前の「cut round」の時刻
loops, last_t, rounds = [], None, 0
for l in err.splitlines():
    m = re.search(r"MIP: cut round (\d+): .*\(([\d.]+)s\)$", l)
    if m:
        last_t = float(m.group(2)); rounds = int(m.group(1)) + 1
    m = re.search(r"MIP: cut loop done: (\d+) cuts added, (\d+) in LP.*obj (\S+) -> (\S+)", l)
    if m:
        loops.append({"added": int(m.group(1)), "in_lp": int(m.group(2)), "obj0": float(m.group(3)), "obj1": float(m.group(4)), "t": last_t, "rounds": rounds})
        last_t, rounds = None, 0
iv = [l for l in err.splitlines() if l.startswith("SEP mushagg")][:3]
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"{cfg}_{p}_{tl}_r{rep}.err.gz", "wt") as f:
    f.write(err)
rec = {"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "wall": round(time.time() - t0, 1), "result": res, "root_loops": loops[:4], "intvub": iv[:2],
       "warn": err.count("MIP: WARNING"), "at": time.strftime("%H:%M:%S")}
with open(OUT, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), loops[:1], iv[:1], flush=True)
