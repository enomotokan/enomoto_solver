"""RENS のやり直しを本当にノード上限まで進んだときだけにする試作 (ENOMOTO_MIP_RENS_MORE_NODES_REAL) の確かめ (2026-11-01、4 スレッド・60 秒、
ソルバーは同時に 1 つだけ)。
使い方: python run.py <設定名,...> <問題,...> <回数> [回の始め] [ログのディレクトリ名 (既定 logs)] [追加の環境変数 K=V ...]
ログは <ログのディレクトリ>/<問題>_<設定>_<回>.log。問題・回ごとに設定の順を逆にする。def は環境変数なし (今の既定 = cand_default_20261031)。"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
REAL = {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"}
CFGS = {
    "def": {},
    "real": dict(REAL),
    # 深さ 2 以上 (サブ MIP の中) ではやり直さない
    "real_s0": dict(REAL, ENOMOTO_MIP_RENS_MORE_NODES_SUB="0"),
    # 深さ 2 以上のやり直しのノード上限を 2 倍にする (外側は 4 倍のまま)
    "real_s2": dict(REAL, ENOMOTO_T_MIP_RENS_MORE_NODES_SUB_MULT="2"),
    # 本当に上限まで進んでいなくてもやり直すが、深さ 2 以上ではやり直さない (比べ用)
    "def_s0": {"ENOMOTO_MIP_RENS_MORE_NODES_SUB": "0"},
}
OFF = set().union(*CFGS.values())

if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    r0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    L = D / (sys.argv[5] if len(sys.argv) > 5 else "logs")
    extra = dict(a.split("=", 1) for a in sys.argv[6:])
    L.mkdir(exist_ok=True)
    for rep in range(r0, r0 + reps):
        for i, p in enumerate(probs):
            order = cfgs if (rep + i) % 2 == 0 else cfgs[::-1]
            for c in order:
                out = L / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if k not in OFF}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c], **extra)
                t0 = time.time()
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(f"# cfg={c} env={CFGS[c]} extra={extra} wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
                print(out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
