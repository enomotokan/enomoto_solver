"""根のカットのループの止め方の試作の計測 (2026-11-01、4 スレッド・60 秒、ソルバーは同時に 1 つ、ENOMOTO_MIP_LOG=1)。
使い方: python run.py <設定名,...> <問題,...> <回数> [時間上限 (秒、既定 60)]。
ログは logs/<問題>_<設定>_<回>.log、1 行の結果は results.jsonl に足す (既にあるログの回は飛ばす)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
CFGS = {
    "def": {},
    # (a) 根の LP の値から動かないラウンドが最初から 2 回続いたら止める
    "fl2": {"ENOMOTO_T_MIP_CUT_FLAT_STOP": "2"},
    "fl3": {"ENOMOTO_T_MIP_CUT_FLAT_STOP": "3"},
    # (b) 1 回の LP の上限に当たったら止める (flat: 根の LP から動いていないときだけ)
    "lcs": {"ENOMOTO_MIP_CUT_LP_CAP_STOP": "1"},
    "lcsf": {"ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat"},
    # (a) + (b)
    "fl3lcsf": {"ENOMOTO_T_MIP_CUT_FLAT_STOP": "3", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat"},
    # (c) 手間の上限 (根のカットの上限・1 回の LP の上限とも)
    "wk": {"ENOMOTO_T_MIP_CUT_WORK_CAP": "1", "ENOMOTO_T_MIP_CUT_LP_WORK": "0.5"},
    "wk2": {"ENOMOTO_T_MIP_CUT_WORK_CAP": "2", "ENOMOTO_T_MIP_CUT_LP_WORK": "0.5"},
    # (c) で時間の安全網を 4 倍に広げ、1 回の LP を反復 max(0.1 x 根の LP, 1000) で切る (時間の際に頼らない形)
    "wkn": {"ENOMOTO_T_MIP_CUT_WORK_CAP": "1", "ENOMOTO_T_MIP_CUT_WORK_TIME_MULT": "4", "ENOMOTO_T_MIP_CUT_LP_WORK": "0.1"},
    # 上 + (b) の flat (反復の上限で LP を外したら、根の LP から動いていなければ止める)
    "wkd": {"ENOMOTO_T_MIP_CUT_WORK_CAP": "1", "ENOMOTO_T_MIP_CUT_WORK_TIME_MULT": "4", "ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat"},
    # wkd から手間の上限を除いた形: 1 回の LP を反復 max(0.1 x 根の LP, 1000) で切り (時間は 5 倍の安全網)、外したら根の LP から
    # 動いていなければ止める。根のカットの時間の上限は既定のまま
    "lpw": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat"},
    # 上 + 根のカットの時間の上限をループの始めから数える (既存の切り替え)
    "lpwc": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    # ここから 2 つ目のビルド (`ENOMOTO_T_MIP_CUT_LP_WORK_PREV` を足したもの)。lpw・lpwc に、反復の上限をこのループで最適まで解けた
    # カットの LP の反復の最大の 5 倍より小さくしない形を足す
    "lpwp": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "5", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat"},
    "lpwpc": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "5", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    # 2 つ目のビルドの既定 (振る舞いは 1 つ目と同じはず)
    "def2": {},
}
def run(cfg, prob, rep, tl):
    (D / "logs").mkdir(exist_ok=True)
    out = D / "logs" / f"{prob}_{cfg}_{rep}.log"
    if out.exists():
        return
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[cfg])
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{prob}.mps"),
                        "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 5 + 200)
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
