"""physiciansched6-2 の解ける回と解けない回を分ける計測 (2026-11-01、4 スレッド・60 秒、ソルバーは同時に 1 つだけ、ENOMOTO_MIP_LOG=1)。
使い方: python run.py <設定名,...> <問題,...> <回数> [初めの回]。設定は CFGS の名前か "名前=変数=値;変数=値" の形。
ログは logs/<問題>_<設定>_<回>.log、結果は res.jsonl に 1 行ずつ足す (同じ名前のログがあれば飛ばす)。
親の環境の ENOMOTO_* は外してから設定の変数を入れる。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
C = WT / ".miplib_cache"
CFGS = {
    "def": {},
    # 試作: 裏のカットの待ちで、ラウンドが 2 つそろうまでは「停滞」とみなさない
    "mr2": {"ENOMOTO_T_MIP_BG_CUTS_WAIT_MIN_ROUNDS": "2"},
    # 試作: 裏のカットの写しの根の LP に、主の基底の列番号と列の値を渡す
    "bx": {"ENOMOTO_MIP_BG_CUTS_BASIS_X": "1"},
}
def cfg(name):
    if name in CFGS:
        return name, CFGS[name]
    n, rest = name.split("=", 1)
    return n, dict(kv.split("=", 1) for kv in rest.split(";"))
if __name__ == "__main__":
    cfgs = [cfg(c) for c in sys.argv[1].split(",")]
    probs, reps = sys.argv[2].split(","), int(sys.argv[3])
    rep0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    (D / "logs").mkdir(exist_ok=True)
    sys.path.insert(0, str(WT / "scripts"))
    import run_miplib_benchmark as rb
    for p in probs:
        rb.fetch(p, C)
    for rep in range(rep0, rep0 + reps):
        for p in probs:
            for c, envc in (cfgs if rep % 2 == 0 else cfgs[::-1]):
                out = D / "logs" / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **envc)
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                lines = [l for l in r.stdout.splitlines() if l.startswith("{")]
                try:
                    res = json.loads(lines[-1])
                except (ValueError, IndexError):
                    res = {"status": "error"}
                rec = {"problem": p, "config": c, "rep": rep, **{k: res.get(k) for k in ("status", "objective", "bound", "time", "nodes", "root_bound", "first_solution_time")}}
                with open(D / "res.jsonl", "a") as f:
                    f.write(json.dumps(rec) + "\n")
                print(json.dumps(rec), flush=True)
