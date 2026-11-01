"""外側の問題を設定を変えて解く (2026-11-01、既定 4 スレッド・60 秒、ソルバーは同時に 1 つ。analysis/neos960392_20261101/cfgs/run.py から)。
使い方: python run.py <設定名,...> <問題,...> <回数> [時間上限 (秒、既定 60)]。設定名は CFGS の名か `n=<文字>` (ENOMOTO_MIP_SUBMIP_NESTED)。
ログは logs/<問題>_<設定>[_t<秒>]_<回>.log、1 行の結果は logs/results.jsonl に足す。
設定名の後ろに @<スレッド数> を付けるとスレッド数を変える (例: def@8)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
WT = D.parents[2]
C = WT / ".miplib_cache"
TRIM3 = {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_RESUME": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"}
CFGS = {
    "def": {},
    # 試作 ENOMOTO_MIP_SUBMIP_NESTED (深さ 1 のサブ MIP の中でもサブ MIP のヒューリスティクスを回す)
    "nc": {"ENOMOTO_MIP_SUBMIP_NESTED": "c"},
    "nclv": {"ENOMOTO_MIP_SUBMIP_NESTED": "clv"},
    "nclvi": {"ENOMOTO_MIP_SUBMIP_NESTED": "clvi"},
}
def cfg_env(name):
    """CFGS の名か、`n=<文字>` (ENOMOTO_MIP_SUBMIP_NESTED=<文字>) に `+変数=値` を続けた名"""
    base, *rest = name.split("+")
    env = dict(CFGS[base]) if base in CFGS else {"ENOMOTO_MIP_SUBMIP_NESTED": base.split("=", 1)[1]}
    for kv in rest:
        k, v = kv.split("=", 1)
        env[k] = v
    return env
def run(cfg, prob, rep, tl):
    name, _, thr = cfg.partition("@")
    thr = thr or "4"
    tag = cfg.replace("@", "_th").replace("=", "-").replace("+", "_") + (f"_t{tl}" if tl != 60 else "")
    out = D / f"{prob}_{tag}_{rep}.log"
    D.mkdir(exist_ok=True)
    if out.exists():
        return
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS=thr, **cfg_env(name))
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{prob}.mps"),
                        "--time-limit", str(tl), "--threads", thr], cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 5 + 200)
    out.write_text(r.stderr + "\n" + r.stdout)
    last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else "{}"
    try:
        res = json.loads(last)
    except Exception:
        res = {"raw": last[:200]}
    rec = {"prob": prob, "cfg": cfg, "tl": tl, "rep": rep, "wall": round(time.time() - t0, 1), **{k: res.get(k) for k in ("status", "objective", "bound", "time", "nodes") if k in res}}
    with open(D / "results.jsonl", "a") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(json.dumps(rec, ensure_ascii=False), flush=True)
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    tl = int(sys.argv[4]) if len(sys.argv) > 4 else 60
    for rep in range(reps):
        for p in probs:
            for c in cfgs:
                run(c, p, rep, tl)
