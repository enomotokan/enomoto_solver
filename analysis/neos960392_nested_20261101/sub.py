"""書き出した近傍 (/tmp/dump/<名>.mps) を ENOMOTO で設定を変えて解く (1 スレッド・30 秒、ソルバーは同時に 1 つ)。
使い方: python sub.py <近傍名,...> <設定名,...> [回数]。ログは sub/<近傍>_<設定>_<回>.log、結果は sub/results.jsonl。
設定の "as" は根のサブ MIP の形 (深さ 1・1 スレッド・打ち切り値 -35.99997)、"full" は普通の MIP (1 スレッド)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
AS = {"ENOMOTO_MIP_DEBUG_AS_SUBMIP": "1", "ENOMOTO_MIP_CUTOFF": "-35.99997"}
CUT = {"ENOMOTO_MIP_CUTOFF": "-35.99997"}
NOSUB = {"ENOMOTO_MIP_NO_LOCKS_HEUR": "1", "ENOMOTO_MIP_NO_CLIQUE_HEUR": "1", "ENOMOTO_MIP_NO_VBOUNDS_HEUR": "1"}
CFGS = {
    # 根のサブ MIP の形
    "as": AS,
    # 普通の MIP
    "full": {},
    # 普通の MIP + 打ち切り値 (本番のサブ MIP と同じ値)
    "full_cut": CUT,
    # 普通の MIP から部品を抜く
    "full_noclq": {"ENOMOTO_MIP_NO_CLIQUE_HEUR": "1"},
    "full_nolck": {"ENOMOTO_MIP_NO_LOCKS_HEUR": "1"},
    "full_novb": {"ENOMOTO_MIP_NO_VBOUNDS_HEUR": "1"},
    "full_noscip": NOSUB,
    "full_norens": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "0"},
    "full_norens_noscip": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "0", **NOSUB},
    "full_nosubmip": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "0", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "0", **NOSUB, "ENOMOTO_MIP_NO_ROOT_REDCOST_HEUR": "1", "ENOMOTO_MIP_NO_LPFACE": "1"},
    "full_n500": {"ENOMOTO_MIP_NODE_LIMIT": "500"},
    "full_cut_noclq": {**CUT, "ENOMOTO_MIP_NO_CLIQUE_HEUR": "1"},
    # 根のサブ MIP の形 + 試作 (深さ 1 の中でサブ MIP を許す)
}
def extra(name):
    """`as+NESTED=clv` のような名を環境変数に (as の形 + ENOMOTO_MIP_SUBMIP_NESTED=clv)。`full+X=Y` も同じ"""
    base, *rest = name.split("+")
    env = dict(CFGS[base])
    for kv in rest:
        k, v = kv.split("=", 1)
        env[k if k.startswith("ENOMOTO_") else "ENOMOTO_MIP_SUBMIP_" + k] = v
    return env
def run(dump, cfg, rep, tl=30):
    out = D / "sub" / f"{dump}_{cfg.replace('=', '-')}_{rep}.log"
    if out.exists():
        return
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", **extra(cfg))
    t0 = time.time()
    r = subprocess.run([sys.executable, str(D / "run_ours_mps.py"), f"/tmp/dump/{dump}.mps", str(tl), "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 5 + 100)
    out.write_text(r.stderr)
    try:
        res = json.loads(r.stdout.strip().splitlines()[-1])
    except Exception:
        res = {}
    # 最初に -238 (近傍の最適) を見つけた時刻 (ソルバーの時計) と出どころ
    t238 = None
    for line in r.stderr.splitlines():
        if line.startswith("MIP: new incumbent -2.38") and t238 is None:
            t238 = line.strip()[:160]
    rec = {"dump": dump, "cfg": cfg, "rep": rep, "wall": round(time.time() - t0, 1), **res, "first238": t238}
    with open(D / "sub" / "results.jsonl", "a") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(json.dumps(rec, ensure_ascii=False), flush=True)
if __name__ == "__main__":
    dumps, cfgs = sys.argv[1].split(","), sys.argv[2].split(",")
    reps = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    for rep in range(reps):
        for d in dumps:
            for c in cfgs:
                run(d, c, rep)
