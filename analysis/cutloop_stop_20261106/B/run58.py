"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (def) と試作を回 (rep) ごとに 1 回ずつ解き、res58.jsonl に足す。
回ごと・問題ごとに設定の順を入れ替える。ENOMOTO_ の環境変数は消してから設定のものだけを与える (ENOMOTO_MIP_THREADS=4・ENOMOTO_MIP_LOG=1)。
ログから最初の根の情報 (rootinfo.py: カットのラウンド数・木に入った時刻) も残す。途中から再開できる。
使い方: python run58.py <rep> <設定名> [<設定名> ...]"""
import json, os, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {},
    "cs": {"ENOMOTO_MIP_CUTSTALL_GAP": "1"},
    "csn": {"ENOMOTO_MIP_CUTSTALL_GAP": "1n"},
}
rep = int(sys.argv[1]); names = sys.argv[2:]
OUT = D / "res58.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for i, p in enumerate(PROBS):
    for c in (names if (i + rep) % 2 == 0 else names[::-1]):
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **CFGS[c])
        t0 = time.time()
        with tempfile.NamedTemporaryFile("w+", suffix=".err", delete=False) as ef:
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True, timeout=400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                ef.flush(); res["warn"] = open(ef.name, errors="replace").read().count("MIP: WARNING")
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            try:
                root = json.loads(subprocess.run([sys.executable, str(D / "rootinfo.py"), ef.name], capture_output=True, text=True).stdout)
            except Exception:  # noqa: BLE001
                root = None
        os.unlink(ef.name)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "wall": round(time.time() - t0, 1), "root": root, "result": res}) + "\n")
