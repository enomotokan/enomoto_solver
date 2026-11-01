"""neos-2978193-inde で既定にする候補 cand が遅くなる部品を切り分ける (2026-10-31、4 スレッド・60 秒、ソルバーは同時に 1 つだけ)。
使い方: python run.py <設定名,...> <問題,...> <回数> [最初の回]。ログは <問題>_<設定>_<回>.log、結果は run.jsonl に 1 行ずつ足す。
設定は全部環境変数で明示する (def は環境変数なし = 6bfcc89d の既定)。
  cand-X: cand から部品 X を外す。def+X: def に部品 X だけ足す。hs の文字は cand-hs<x> (ENOMOTO_MIP_SUBMIP_HIGHS=-s<x>)。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
PARTS = {  # cand の部品
    "hs": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"},
    "pi": {"ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1"},
    "r0": {"ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0"},
    "bgw": {"ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "trim": {"ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "r"},
    "ff": {"ENOMOTO_T_MIP_RESTART_FIRST_FAC": "0.01"},
    "ps": {"ENOMOTO_MIP_BG_CUTS_PROOF_STOP": "1"},
}
CAND = {k: v for p in PARTS.values() for k, v in p.items()}
CFGS = {"def": {}, "cand": CAND}
for name, p in PARTS.items():
    CFGS[f"cand-{name}"] = {k: v for k, v in CAND.items() if k not in p}
    CFGS[f"def+{name}"] = dict(p)
for ch in "pcgtihf":
    CFGS[f"cand-hs{ch}"] = dict(CAND, ENOMOTO_MIP_SUBMIP_HIGHS=f"-s{ch}")
EXTRA = json.loads(os.environ.get("DIAG_EXTRA_CFGS", "{}"))  # 試作の設定 {"名前": {環境変数}}、"+cand" を含めると cand に足す
for k, v in EXTRA.items():
    base = CAND if v.pop("+cand", None) else {}
    CFGS[k] = dict(base, **v)
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    rep0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    sys.path.insert(0, str(WT / "scripts"))
    import run_miplib_benchmark as rb
    for p in probs:
        rb.fetch(p, C)
    for rep in range(rep0, rep0 + reps):
        for p in probs:
            for c in (cfgs if rep % 2 == 0 else cfgs[::-1]):
                out = D / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if not k.startswith(("ENOMOTO_MIP_", "ENOMOTO_T_MIP_"))}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else "{}"
                with open(D / "run.jsonl", "a") as f:
                    f.write(json.dumps({"prob": p, "cfg": c, "rep": rep, "out": last}, ensure_ascii=False) + "\n")
                print(out.name, last[:160], flush=True)
