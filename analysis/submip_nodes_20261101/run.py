"""サブ MIP のノード上限を下げる形を、得の問題・損の問題で比べる (2026-11-01、4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
使い方: python run.py <設定名,...> <問題,...> <回数> [回の始め]。ログは logs/<問題>_<設定>_<回>.log (先頭に使った環境変数)。
def は環境変数なし (= 今の既定、analysis/cand_default_20261031.md)。設定は `名前=VAR:値;VAR:値` の形でも渡せる (既存の設定名も混ぜられる)。
回ごとに設定の順を逆にする。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
CFGS = {
    "def": {},
    "nm02": {"ENOMOTO_T_MIP_SUBMIP_NODES_MULT": "0.2"},
    "nm01": {"ENOMOTO_T_MIP_SUBMIP_NODES_MULT": "0.1"},
    # hs から `t` (根のサブ MIP の時間の上限を残り時間にする) を抜く (analysis/cand_slow_neos-1582420_20261031.md の手当て)
    "st": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-st"},
    # 以下は試作 (2026-11-01、既定では使わない)
    # 計測のときは `ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP`・`ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL` を `ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES`・
    # `ENOMOTO_MIP_SUBMIP_NOSOL_NODES_SERIAL` と呼んでいた (ログの先頭の env の行はその名前)。別のセッションの試作 `ENOMOTO_MIP_SUBMIP_NOSOL_STOP` の
    # 調整値と重なったので改名した
    # 解を 1 つも見つけないサブ MIP を 100 ノードで止める + RENS のやり直しを本当にノード上限まで進んだときだけにする
    "ns100": {"ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "100", "ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    # ns100 の 2 つの部品を分ける: RENS のやり直しの試作だけ・解のないサブ MIP の上限だけ
    "real": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    "ns100o": {"ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "100"},
    # 解のないサブ MIP の上限を 250 (10teams の根の被約費用のサブ MIP は 233 ノード目に最適解を見つける) + RENS のやり直しの試作
    "ns250": {"ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "250", "ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    # ns250 の上限を、主のスレッドで逐次に解くサブ MIP (と木のスレッドのもの) だけにかける (並列のサブ MIP は今まで通り)
    "ns250s": {"ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "250", "ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL": "1", "ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    # 成果連動の打ち切りの最小ノードを 50 -> 20
    "om20": {"ENOMOTO_T_MIP_SUBMIP_OUTCOME_MIN_NODES": "20"},
    # 入れ子のサブ MIP (サブ MIP の中のサブ MIP) だけノード上限を 0.2 倍
    "deep02": {"ENOMOTO_T_MIP_SUBMIP_NODES_MULT_DEEP": "0.2"},
    # 外側の木の中のサブ MIP だけノード上限を 0.2 倍
    "tree02": {"ENOMOTO_T_MIP_SUBMIP_NODES_MULT_TREE": "0.2"},
}

def parse_extra(name):
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
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                envs = " ".join(f"{a}={b}" for a, b in CFGS[c].items())
                out.write_text(f"# env: {envs}\n" + r.stderr + "\n" + r.stdout)
                print(out.name, r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
