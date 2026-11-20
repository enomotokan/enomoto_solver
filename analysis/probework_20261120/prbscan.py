"""ENOMOTO_MIP_PRBWORK が働く問題の数え上げ (走査だけ)。P (MIPLIB 58 問) と M (Mittelmann、P と重ならない問題) を
ENOMOTO_MIP_PRBWORK=1・1 スレッド・種 1・TL 秒で回し、ログからプロービングの各回 (列の数・固定・締め付け・含意・秒・pwork)、
PRBWORK の差し引き・省いた回、再スタートの回数を拾う。
python prbscan.py [TL=60] [並べる数=4] → PRBSCAN_OUT (既定 prbscan.jsonl、途中から再開できる)"""
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
S = WT / "analysis/scipslow_20261119"
P = sorted({x[0] for s in json.loads((S / "shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
OUT = Path(os.environ.get("PRBSCAN_OUT", str(D / "prbscan.jsonl")))
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 60.0
NJ = int(sys.argv[2]) if len(sys.argv) > 2 else 4
PR = re.compile(r"MIP: probing: (\d+) of (\d+) binaries probed \((\d+) passes\), (\d+) fixed, (\d+) bounds tightened, (\d+) implications \(([\d.]+)s.*pwork (\d+)")


def one(sp):
    s, p = sp
    row = {"set": s, "problem": p}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
        return row
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_PRBWORK": "1"})
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
        out = pr.stderr
        try:
            r = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            row["status"], row["time"] = r.get("status"), r.get("time")
        except Exception:  # noqa: BLE001
            row["status"] = "crash"
    except subprocess.TimeoutExpired as e:
        out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
        row["status"] = "timeout"
    ev = []
    for l in out.splitlines():
        m = PR.search(l)
        if m:
            a = m.groups()
            ev.append(["probe", int(a[0]), int(a[1]), int(a[3]), int(a[4]), int(a[5]), float(a[6]), int(a[7])])
        elif "PRBWORK: no result" in l:
            m = re.search(r"(\d+) work charged, (\d+) left", l)
            ev.append(["charge", int(m.group(1)), int(m.group(2))])
        elif "probing: skipped (PRBWORK" in l:
            ev.append(["skip"])
        elif re.match(r"MIP: restart \d+ \(", l):
            ev.append(["restart"])
        elif "equivalent binary pairs" in l and l.startswith("MIP: probing: "):
            m = re.search(r"probing: (\d+) equivalent binary pairs", l)
            if m:
                ev.append(["equiv", int(m.group(1))])
    row["events"] = ev
    return row


todo = [sp for sp in probs if sp[1] not in done]
with ThreadPoolExecutor(NJ) as ex, OUT.open("a") as f:
    for row in ex.map(one, todo):
        f.write(json.dumps(row, ensure_ascii=False) + "\n")
        f.flush()
        print(row["problem"], row.get("status"), sum(1 for e in row.get("events", []) if e[0] == "skip"), flush=True)
