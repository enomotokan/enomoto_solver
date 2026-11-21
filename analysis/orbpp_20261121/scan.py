"""ORBPP の洗い出し: MIPLIB 2017 の collection (collection.json、tag_collection.html から) のうち、試験集合 (testset.json) の外で
分割・パッキングの印 (set_partitioning / set_packing) があり、非零 30 万以下・easy か hard の問題を、`ENOMOTO_MIP_LOG=1`・1 スレッド・
`ENOMOTO_T_MIP_ORBPP_MIN_COLS=0` で走らせ、対称性の行 (生成元・オービトープ・ORBPP) が出たら止める (前処理と対称性の検出まで)。
python scan.py [並列数] → scan.jsonl に足す (記録済みは飛ばす)。python scan.py [並列数] --wide → scan.jsonl に足す (印を問わず非零 100 万以下)。python scan.py [並列数] --rescan → scan2.jsonl (diag.patch のビルドで、1 回目に対称性まで進んだ問題だけ)。"""
import json, os, re, subprocess, sys, time, threading
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
RESCAN = "--rescan" in sys.argv
OUT = D / ("scan2.jsonl" if RESCAN else "scan.jsonl")
CAP = 180.0
test = set(json.loads((D / "testset.json").read_text()))
coll = json.loads((D / "collection.json").read_text())
cand = [o for o in coll if o["name"] not in test and o["status"] in ("easy", "hard") and {"set_partitioning", "set_packing"} & set(o["tags"]) and (o["nnz"] or 9e9) <= 300000]
cand.sort(key=lambda o: o["nnz"])
WIDE = "--wide" in sys.argv
if WIDE:  # 広げた走査: 印を問わず、非零 100 万以下の easy・hard 全部 (diag.patch のビルドなので ORBDIAG struct の行も出る)
    cand = [o for o in coll if o["name"] not in test and o["status"] in ("easy", "hard") and (o["nnz"] or 9e9) <= 1000000]
    cand.sort(key=lambda o: o["nnz"])
if RESCAN:  # 2 回目: 1 回目 (scan.jsonl) で対称性の行まで進んだ問題だけを、diag.patch のビルドで (オービトープごとの構造の行 ORBDIAG struct)
    sym = {json.loads(l)["problem"] for l in (D / "scan.jsonl").read_text().splitlines() if json.loads(l).get("stop") == "sym"}
    cand = [o for o in cand if o["name"] in sym]
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
lock = threading.Lock()
KEEP = re.compile(r"MIP: symmetry|MIP: presolve|ORBPP|orbitope")


def one(o):
    p = o["name"]
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row = {"problem": p, "error": f"fetch: {e}"[:200]}
    else:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_ORBPP_MIN_COLS": "0", "ENOMOTO_MIP_ORBDIAG": "1"})
        t0 = time.time()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(CAP), "--threads", "1"],
                              cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        lines, stop = [], None
        timer = threading.Timer(CAP + 30, pr.kill); timer.start()
        for l in pr.stderr:
            if KEEP.search(l): lines.append(l.rstrip()[:600])
            if re.match(r"MIP: symmetry: \d+ generators, ", l): stop = "sym"; break
            if "MIP: symmetry: none" in l: stop = "none"; break
            if l.startswith("MIP: solving the root LP"): stop = "root"; break
        pr.kill(); timer.cancel(); pr.wait()
        row = {"problem": p, "stop": stop, "secs": round(time.time() - t0, 1), "lines": lines}
    row.update({k: o[k] for k in ("status", "vars", "bins", "ints", "conts", "cons", "nnz", "group", "tags")})
    with lock, open(OUT, "a") as f:
        f.write(json.dumps(row) + "\n")
    print(p, row.get("stop"), row.get("secs"), [l for l in row.get("lines", []) if "orbitope" in l][:2], flush=True)


with ThreadPoolExecutor(int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1].isdigit() else 3) as ex:
    list(ex.map(one, [o for o in cand if o["name"] not in done]))
