"""1 回の計測 (analysis/prechain_20261110.md、symdyn_20261110/run.py を写したもの): ENOMOTO を解き、結果と最初の暫定解の時刻・前処理の時間・
連鎖の要約を jsonl に足し、詳細ログを logs/<設定>_<問題>_<時間>_r<回>.err.gz に置く。ENOMOTO_ の環境変数は消してから設定のものだけを与える。
使い方: python run.py 設定 問題 時間 回 出力 [名前=値 ...] (設定は CFGS のキー。回は ENOMOTO_T_MIP_SEED にも使う)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    # (a) 前処理の途中から FJ・Local-MIP を回し、根まで続ける
    "e": {"ENOMOTO_MIP_PRECHAIN": "e"},
    # (a) + (b) 境界の受け渡し + (c) 暫定解の受け渡し
    "ebi": {"ENOMOTO_MIP_PRECHAIN": "ebi"},
    "ei": {"ENOMOTO_MIP_PRECHAIN": "ei"},
    "eb": {"ENOMOTO_MIP_PRECHAIN": "eb"},
    # 並列のプロービングと分け合う
    "ebip": {"ENOMOTO_MIP_PRECHAIN": "ebip", "ENOMOTO_MIP_PREPAR": "ple"},
    "ple": {"ENOMOTO_MIP_PREPAR": "ple"},
    # 根でも 2 本残す
    "ebi2": {"ENOMOTO_MIP_PRECHAIN": "ebi", "ENOMOTO_T_MIP_PRECHAIN_ROOT_KEEP": "2"},
}
cfg, p, tl, rep, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
OUT = D / out
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_PRETIME="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[cfg])
# 診断用: 6 番目以降の引数 `名前=値` を足す
env.update(dict(a.split("=", 1) for a in sys.argv[6:]))
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
def first(pat):
    for l in err.splitlines():
        m = re.search(pat, l)
        if m:
            return m
    return None
# 最初の暫定解の時刻: 前処理の始めから数えた壁時計が要るので、`presolve finished` の秒と MIP の時計の秒を足す
pre = first(r"PRETIME total=([\d.]+)")
inc = first(r"MIP: new incumbent .*?\(([\d.]+)s")
chain = first(r"PRECHAIN summary: found (\d+) improvements \(first at ([-\d.s]+)\), dropped (\d+) .*?injected (\d+), last tightened bounds (\d+)")
rec = {"problem": p, "cfg": cfg, "tl": tl, "rep": rep, "wall": round(time.time() - t0, 1), "result": res,
       "pre": float(pre.group(1)) if pre else None, "inc": float(inc.group(1)) if inc else None,
       "chain": list(chain.groups()) if chain else None,
       "warn": err.count("MIP: WARNING"), "at": time.strftime("%H:%M:%S")}
with open(OUT, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(p, cfg, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), rec["pre"], rec["inc"], rec["chain"], flush=True)
