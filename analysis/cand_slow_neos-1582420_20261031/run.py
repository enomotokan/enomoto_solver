"""cand で neos-1582420 が遅くなる理由の切り分け (2026-10-31、4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
使い方: python run.py <設定名,...> <問題,...> <回数> [回の始め]。ログは <問題>_<設定>_<回>.log。
設定は環境変数で全部明示する (def は環境変数なし = 6bfcc89d の既定)。cand の部品を 1 つずつ外した形は `c-x`、def に 1 つずつ足した形は `d+x`。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
# cand の部品
P = {
    "hs": ("ENOMOTO_MIP_SUBMIP_HIGHS", "-s"),
    "pi": ("ENOMOTO_MIP_SUBMIP_PC_INHERIT", "1"),
    "r0": ("ENOMOTO_T_MIP_SUBMIP_RELIABLE", "0"),
    "bgw": ("ENOMOTO_MIP_BG_CUTS_WAIT", "1"),
    "trim": ("ENOMOTO_MIP_SUBMIP_HIGHS_TRIM", "r"),
    "ff": ("ENOMOTO_T_MIP_RESTART_FIRST_FAC", "0.01"),
    "ps": ("ENOMOTO_MIP_BG_CUTS_PROOF_STOP", "1"),
}
CAND = dict(P.values())
CFGS = {"def": {}, "cand": dict(CAND)}
for k, (e, _) in P.items():
    CFGS[f"c-{k}"] = {a: b for a, b in CAND.items() if a != e}
    CFGS[f"d+{k}"] = {P[k][0]: P[k][1]}
for ch in "pcgtihf":  # hs の文字を 1 つずつ抜く (cand の中で)
    CFGS[f"c-{ch}"] = dict(CAND, ENOMOTO_MIP_SUBMIP_HIGHS="-s" + ch)
EXTRA = {  # 手当ての試作
    # 解のないうちの根のサブ MIP は `t` のない時間の上限で止める (`ENOMOTO_MIP_SUBMIP_HIGHS_NOSOL`、倍率 1・2)
    "nosol": dict(CAND, ENOMOTO_MIP_SUBMIP_HIGHS_NOSOL="1"),
    "nosol2": dict(CAND, ENOMOTO_MIP_SUBMIP_HIGHS_NOSOL="1", ENOMOTO_T_MIP_SUBMIP_HIGHS_NOSOL_MULT="2"),
}
CFGS.update(EXTRA)

def parse_extra(name):
    """`名前=VAR:値;VAR:値` の形で追加の設定を受ける (台本を書き換えずに試すため)。"""
    n, spec = name.split("=", 1)
    base = {}
    for kv in spec.split(";"):
        if kv in CFGS:
            base.update(CFGS[kv]); continue
        a, b = kv.split(":", 1); base[a] = b
    CFGS[n] = base
    return n

if __name__ == "__main__":
    cfgs = [parse_extra(c) if "=" in c else c for c in sys.argv[1].split(",")]
    probs, reps = sys.argv[2].split(","), int(sys.argv[3])
    r0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    for rep in range(r0, r0 + reps):
        order = cfgs if rep % 2 == 0 else cfgs[::-1]
        for p in probs:
            for c in order:
                out = D / "logs" / f"{p}_{c}_{rep}.log"
                out.parent.mkdir(exist_ok=True)
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                envs = " ".join(f"{a}={b}" for a, b in CFGS[c].items())
                out.write_text(f"# env: {envs}\n" + r.stderr + "\n" + r.stdout)
                print(out.name, r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
