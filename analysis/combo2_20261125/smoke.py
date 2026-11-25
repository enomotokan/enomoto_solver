"""回す前の確かめ (README 3 節): 小さい問題で c2a・c2b の各変数が効いているかを見る (analysis/combo_20261124/smoke.py の写し)。
    python analysis/combo2_20261125/smoke.py [--problems p0201,misc07,noswot,nw04,app1-1] [--tl 60]
- 1 スレッド・種 1 で、既定・各変数だけ・combo を解き (`ENOMOTO_MIP_LOG=1`・診断 `ENOMOTO_MIP_CUTSEL_PROBE_TIE_LOG=1`)、状態・時間・ノード・LP の反復と、
  各試作のログの行の数を出す。1 スレッドは決定的なので、(ノード, LP の反復) が既定と違えば、その変数で道筋が変わった (= 効いている)。
- 診断 `ENOMOTO_MIP_SBADAPT_DIAG=1` (解き方は変えない) で SB_ADAPT の特徴の行 (`SBADAPT_FEAT`) も数える。FP の第 3 段 (`FP stage 3: ` はサブ MIP の形、`FP stage 3 LS` は FP_STAGE3_LS の形)。
- ログの行のある試作: SB_PROBEBD (`node bounds from both children` の数)、HEUR_ADAPT (`HEURADAPT root`)、NODEROWS_ADAPT (`node rows adapt`)、
  ENUM_ADAPT (`ENUM_ADAPT:`)、TIE_FEWER (`CUTSEL_TIE: tie true` の行 = 比べのラウンドで伸びが同点だった回)。CUTOFF_USER_TREE・NODELP_BFRT_GROUP はログの行がないので道筋の違いで見る。
- PROOFPROP=i は 2 スレッド以上でだけ働く (4 スレッドは決定的でないので、ここでは道筋で見ない。コードの条件で確かめる)。
ログは logs/ (コミットしない)。"""
import argparse, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import C2  # noqa: E402
ap = argparse.ArgumentParser(); ap.add_argument("--problems", default="p0201,misc07,noswot,nw04,app1-1"); ap.add_argument("--tl", default="60")
a = ap.parse_args()
# 各変数だけ (FP_STAGE3_LS は HEUR_ADAPT が第 3 段を開くので HEUR_ADAPT と組にする)、SB_ADAPT だけ、c2a・c2b
one = {k: v for k, v in C2.items() if k not in ("ENOMOTO_MIP_PROOFPROP", "ENOMOTO_MIP_FP_STAGE3_LS")}
CF = ({"def": {}} | {k.replace("ENOMOTO_MIP_", ""): {k: v} for k, v in one.items()}
      | {"HEUR_ADAPT+FP_STAGE3_LS": {"ENOMOTO_MIP_HEUR_ADAPT": C2["ENOMOTO_MIP_HEUR_ADAPT"], "ENOMOTO_MIP_FP_STAGE3_LS": C2["ENOMOTO_MIP_FP_STAGE3_LS"]}}
      | {"SB_ADAPT": {"ENOMOTO_MIP_SB_ADAPT": "1"}, "c2a": dict(C2), "c2b": dict(C2, ENOMOTO_MIP_SB_ADAPT="1"), "c2c": dict(C2, ENOMOTO_MIP_SB_PROBEBD="1")})
PAT = {"sbpb": r"node bounds from both children (\d+)", "heurad": r"^HEURADAPT root", "nra": r"node rows adapt", "enumad": r"ENUM_ADAPT:", "tie": r"CUTSEL_TIE: tie true",
       "fp3": r"FP stage 3: ", "fp3ls": r"FP stage 3 LS", "sbad": r"^SBADAPT_FEAT"}
(D / "logs").mkdir(exist_ok=True)
print("| 問題 | 設定 | 状態 | 秒 | ノード | LP 反復 | 既定と道筋 | " + " | ".join(PAT) + " |")
print("|---|---|---|---:|---:|---:|---|" + "---:|" * len(PAT))
for p in a.problems.split(","):
    mps = rb.fetch(p, WT / ".miplib_cache"); ref = None
    for name, ev in CF.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ev); env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CUTSEL_PROBE_TIE_LOG="1", ENOMOTO_MIP_SBADAPT_DIAG="1")
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", a.tl, "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True)
        (D / "logs" / f"{p}_{name}.log").write_text(pr.stderr)
        try: r = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: r = {"status": "crash"}  # noqa: BLE001
        key = (r.get("nodes"), r.get("lp_iterations"))
        if name == "def": ref = key
        cnt = {}
        for k, pat in PAT.items():
            ms = [m for m in (re.search(pat, l) for l in pr.stderr.splitlines()) if m]
            cnt[k] = (max(int(m[1]) for m in ms) if k == "sbpb" and ms else len(ms))
        print(f"| {p} | {name} | {r.get('status')} | {r.get('time') or 0:.1f} | {r.get('nodes')} | {r.get('lp_iterations')} | {'同じ' if key == ref else '違う'} | "
              + " | ".join(str(cnt[k]) for k in PAT) + " |", flush=True)
