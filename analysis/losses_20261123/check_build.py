"""入れた当方の wheel が、計測に使う版と同じ動きをするかの照合: python check_build.py [--write]
.so の sha1 は作った機械のリポジトリの場所・CARGO_HOME の場所 (RUSTFLAGS の -Cprofile-use のパスと、依存クレートのパスの文字列) で変わるので、
1 スレッド (決定的)・種 1 で 3 問を解いた (状態, 目的値, ノード数) を ref_check.json と比べる。--write で今の版の値を ref_check.json に書く。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["khb05250", "air03", "neos8"]
out = {}
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                       cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    out[p] = {"status": res["status"], "objective": res["objective"], "nodes": res["nodes"]}
    print(p, out[p], f"{res['time']:.1f} 秒")
ref_f = D / "ref_check.json"
if "--write" in sys.argv:
    ref_f.write_text(json.dumps(out, indent=1)); print("ref_check.json に書いた")
else:
    ref = json.loads(ref_f.read_text())
    bad = [p for p in PROBS if ref[p]["status"] != out[p]["status"] or ref[p]["nodes"] != out[p]["nodes"]
           or abs((ref[p]["objective"] or 0) - (out[p]["objective"] or 0)) > 1e-6 * max(1, abs(ref[p]["objective"] or 0))]
    print("一致 (同じ動き)" if not bad else f"食い違い: {bad} (ソース・rustc・学習結果・フラグのどれかが違う)")
