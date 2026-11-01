"""cand (既定にする候補) で n5-3 などが遅くなる理由の切り分け (2026-10-31、4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
使い方: python run.py <設定名,...> <問題,...> <回数> [回の始め]。ログは logs/<問題>_<設定>_<回>.log。
設定はどれも環境変数で明示する (def は環境変数なし = 6bfcc89d の既定。ほかのセッションが cand を既定にしても、比べる中身が変わらないように
cand の部品は全部明示し、def は部品を外した値を明示する形 `def_x` も用意する)。"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
L = D / "logs"
# cand の部品 (名前 → 環境変数)。hs の文字は ENOMOTO_MIP_SUBMIP_HIGHS=-s から 1 文字ずつ抜く形で別に扱う
PARTS = {
    "pi": {"ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1"},
    "r0": {"ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0"},
    "bgw": {"ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "trim": {"ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "r"},
    "ff": {"ENOMOTO_T_MIP_RESTART_FIRST_FAC": "0.01"},
    "ps": {"ENOMOTO_MIP_BG_CUTS_PROOF_STOP": "1"},
}
HS = {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"}
# 6bfcc89d の既定の値 (cand の部品を外したときにはこれを明示する。既定が cand になっても def の意味を保つため)
OFF = {
    "ENOMOTO_MIP_SUBMIP_HIGHS": None,  # 既定は未設定 (hs の形を使わない)
    "ENOMOTO_MIP_SUBMIP_PC_INHERIT": None,
    "ENOMOTO_T_MIP_SUBMIP_RELIABLE": None,
    "ENOMOTO_MIP_BG_CUTS_WAIT": None,
    "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": None,
    "ENOMOTO_T_MIP_RESTART_FIRST_FAC": None,
    "ENOMOTO_MIP_BG_CUTS_PROOF_STOP": None,
}


def cand(**over):
    e = dict(HS)
    for p in PARTS.values():
        e.update(p)
    e.update(over)
    return e


CFGS = {"def": {}, "cand": cand()}
# cand から部品を 1 つ外す (cand-pi など)
for k in PARTS:
    CFGS[f"cand-{k}"] = {kk: vv for kk, vv in cand().items() if kk not in PARTS[k]}
CFGS["cand-hs"] = {kk: vv for kk, vv in cand().items() if kk not in HS}
# cand の hs から文字を 1 つ抜く (cand-hp: ENOMOTO_MIP_SUBMIP_HIGHS=-sp)
for ch in "pcgtihf":
    CFGS[f"cand-h{ch}"] = cand(ENOMOTO_MIP_SUBMIP_HIGHS="-s" + ch)
# def に部品を 1 つ足す
for k in PARTS:
    CFGS[f"def+{k}"] = dict(PARTS[k])
CFGS["def+hs"] = dict(HS)

# 手当ての試作 (このディレクトリの記録の 4 節)
REAL = {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"}
CFGS["cand+real"] = cand(**REAL)
CFGS["def+real"] = dict(REAL)
CFGS["cand+real+dc"] = cand(**REAL, ENOMOTO_MIP_SUBMIP_HIGHS_DEEP="-c")
CFGS["cand+real+dch"] = cand(**REAL, ENOMOTO_MIP_SUBMIP_HIGHS_DEEP="-ch")
CFGS["cand+dc"] = cand(ENOMOTO_MIP_SUBMIP_HIGHS_DEEP="-c")
OFF.update({k: None for k in REAL})
OFF["ENOMOTO_MIP_SUBMIP_HIGHS_DEEP"] = None
TM8 = {"ENOMOTO_T_MIP_SUBMIP_HIGHS_TIME_MAX": "8"}
CFGS["cand+tm8"] = cand(**TM8)
CFGS["cand+real+dc+tm8"] = cand(**REAL, ENOMOTO_MIP_SUBMIP_HIGHS_DEEP="-c", **TM8)
OFF.update({k: None for k in TM8})

if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    r0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    L.mkdir(exist_ok=True)
    for rep in range(r0, r0 + reps):
        for i, p in enumerate(probs):
            # 問題・回ごとに設定の順を逆にする (時刻の偏りを打ち消す)
            order = cfgs if (rep + i) % 2 == 0 else cfgs[::-1]
            for c in order:
                out = L / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if k not in OFF}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                t0 = time.time()
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(f"# cfg={c} env={CFGS[c]} wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
                print(out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
