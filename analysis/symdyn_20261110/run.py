"""1 回の計測 (analysis/symdyn_20261110.md、redsplit_20261107/run.py を写したもの): ENOMOTO を解き、結果と対称性の数 (SYMDYN・
orbital fixing・オービトープの固定・SYMREFL) を jsonl に足し、詳細ログを logs/<設定>_<問題>_<時間>_r<回>.err.gz に置く。
ENOMOTO_ の環境変数は消してから設定のものだけを与える。
使い方: python run.py 設定 問題 時間 回 出力  (設定は CFGS のキー。回は ENOMOTO_T_MIP_SEED にも使う)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    # (a) 動的な辞書式の縮小を orbital fixing の代わりに (オービトープは既定のまま)
    "dyn": {"ENOMOTO_MIP_SYMDYN": "1"},
    # (a') オービトープを使わず、全ての生成元を動的な辞書式の縮小に
    "dynall": {"ENOMOTO_MIP_SYMDYN": "1", "ENOMOTO_MIP_NO_ORBITOPE": "1"},
    # 比べ用: オービトープを使わず全ての生成元を orbital fixing に
    "ofall": {"ENOMOTO_MIP_NO_ORBITOPE": "1"},
    # (b) 反転 (SYMDYN も入る)
    "refl": {"ENOMOTO_MIP_SYMREFL": "1"},
}
cfg, p, tl, rep, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
OUT = D / out
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[cfg])
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
(D / "logs").mkdir(exist_ok=True)
with gzip.open(D / "logs" / f"{cfg}_{p}_{tl}_r{rep}.err.gz", "wt") as f:
    f.write(err)
def nums(pat):
    out = []
    for l in err.splitlines():
        m = re.search(pat, l)
        if m:
            out.append([float(x) for x in m.groups()])
    return out
sym = {
    "symdyn": nums(r"SYMDYN \((\d+) permutations, (\d+) reflections\): tightened (\d+) bounds at (\d+) nodes, pruned (\d+) nodes, strict (\d+)"),
    "of": nums(r"orbital fixing \((\d+) generators\): fixed (\d+) bounds at (\d+) nodes, pruned (\d+) nodes"),
    "refl": nums(r"SYMREFL (\d+) reflection generators"),
    "gens": nums(r"symmetry: (\d+) generators"),
}
rec = {"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "wall": round(time.time() - t0, 1), "result": res, "sym": sym,
       "warn": err.count("MIP: WARNING"), "at": time.strftime("%H:%M:%S")}
with open(OUT, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), sym["symdyn"][-1:] or sym["of"][-1:], flush=True)
