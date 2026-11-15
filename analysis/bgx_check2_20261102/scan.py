"""1. 洗い出し: 既定 (環境変数なし) で各問題を 1 回解き、`MIP: background cuts: basis with the column values passed/not passed (root LP X s)`
の行が出たら止める (行が出るのは主の根のカットのループの後。そこまでのログだけで足りる)。4 スレッド・時間の上限 60 秒、ENOMOTO_MIP_LOG=1。
出力 scan.jsonl (問題・passed/not passed/なし・主の根の LP の秒・行が出た時刻)。ログは scan_logs/ (行が出たところまで)。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from plist import ALL, WT  # noqa: E402
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
out = D / "scan.jsonl"
done = {json.loads(l)["prob"] for l in out.read_text().splitlines()} if out.exists() else set()
(D / "scan_logs").mkdir(exist_ok=True)
pat = re.compile(r"background cuts: basis with the column values (passed|not passed) \(root LP ([0-9.]+)s\)")
for p in (sys.argv[1].split(",") if len(sys.argv) > 1 else ALL):
    if p in done:
        continue
    # 別に先取りしている取得 (環境変数 BGX2_PREFETCH_LOG の記録) があれば、その問題を取り終えるまで待つ
    pl = os.environ.get("BGX2_PREFETCH_LOG")
    while pl and not any(l.split()[1:2] == [p] for l in Path(pl).read_text().splitlines()):
        time.sleep(2)
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"prob": p, "res": "fetch error", "err": str(e)}), file=out.open("a"), flush=True)
        continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    lines, rec = [], {"prob": p, "res": "none"}
    for l in pr.stdout:
        lines.append(l)
        m = pat.search(l)
        if m:
            rec = {"prob": p, "res": m.group(1), "root_lp": float(m.group(2)), "at": round(time.time() - t0, 1)}
            pr.kill()
            break
        if time.time() - t0 > 90:
            pr.kill()
            rec["res"] = "timeout"
            break
    pr.wait()
    if rec["res"] == "none":
        st = [l for l in lines if l.startswith("{")]
        rec["final"] = json.loads(st[-1])["status"] if st else None
        m2 = [re.search(r"root LP ([0-9.]+)s", l) for l in lines if "restart check" in l]
        rec["root_lp_restart_check"] = float(m2[0].group(1)) if m2 and m2[0] else None
        rec["bg_lines"] = sum("background cuts" in l for l in lines)
    rec["wall"] = round(time.time() - t0, 1)
    (D / "scan_logs" / f"{p}.log").write_text("".join(lines))
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec), flush=True)
