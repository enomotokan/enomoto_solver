"""combo の計測の分担を回す: python analysis/combo_20261124/run.py K (shards.json の K 番目の (問題, 設定, 種, 秒) を順に解き、res_sK.jsonl に足す)。
止めても同じコマンドで続きから回る (同じ (問題, 設定, 種) の行があれば飛ばす。crash・取得失敗の行は次の回にやり直す。
ただし外から -9 で殺された crash (台のメモリ不足の OOM) が 2 回そろった回は、crash で確定してやり直さない)。
ソルバーは同時に 1 つ (4 スレッドの回は 4 コアを使うので、ほかの計測と並べない)。上限 + 300 秒を過ぎたら打ち切って killed とする。
行の形は analysis/protoscreen_20261124/run.py と同じ (この台本はその写し)。設定は cfg.py。名前が _t1 で終われば 1 スレッド、_t4 なら 4 スレッド。

分担の組み替え (README 7 節): takeover.json に {"K": N} があれば、分担 K の終わりの N 組 (def と combo の 2 回で 1 組) を手伝いに渡す。
- 持ち主 (run.py K): 渡した N 組を回さない。takeover.json は始めるときに読む (書き換えたら、取り込んでから同じコマンドで回し直す)。
- 手伝い (run.py K --tail): 渡された N 組だけを、終わりの組から逆順に res_sK_tail.jsonl に足す。止めても同じコマンドで続きから回る。
takeover.json がなければ、これまでと同じ動き。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
TAIL = "--tail" in sys.argv
argv = [x for x in sys.argv if x != "--tail"]
k = int(argv[1])
SH = Path(argv[2]) if len(argv) > 2 else D / "shards.json"
mine = json.loads(SH.read_text())["shards"][k]
TO = D / "takeover.json"
n_tail = 2 * int(json.loads(TO.read_text()).get(str(k), 0)) if TO.exists() else 0
assert len(mine) % 2 == 0 and n_tail <= len(mine)
if TAIL:
    assert n_tail > 0, f"takeover.json で分担 {k} の組が渡されていない"
    # 手伝いは終わりの組から逆順に回す (持ち主が回し直す前に渡した組へ進んでも、重なりが途中の数組で済む)。組の中の順はそのまま
    tail = mine[len(mine) - n_tail:]
    mine = [r for i in range(len(tail) - 2, -1, -2) for r in tail[i:i + 2]]
else:
    mine = mine[:len(mine) - n_tail]
OUT = Path(argv[3]) if len(argv) > 3 else D / (f"res_s{k}_tail.jsonl" if TAIL else f"res_s{k}.jsonl")
done = set()
oom = {}  # 外から -9 で殺された crash (台のメモリ不足の OOM) の回数。2 回そろった回は crash で確定し、やり直さない (README 8 節)
# 持ち主と手伝いの両方の行を見る (渡す前に持ち主が回した組・渡した後で手伝いが回した組を、もう一度回さない)
for F in {OUT, D / f"res_s{k}.jsonl", D / f"res_s{k}_tail.jsonl"}:
    if not F.exists(): continue
    for l in F.read_text().splitlines():
        if not l.strip(): continue
        r = json.loads(l); key = (r["problem"], r["cfg"], r["seed"])
        if r["result"].get("status") not in ("crash", "fetch_error"):
            done.add(key)
        elif r["result"].get("status") == "crash" and r["result"].get("returncode") == -9:
            oom[key] = oom.get(key, 0) + 1
            if oom[key] >= 2: done.add(key)
try:
    COMMIT = subprocess.run(["git", "-C", str(WT), "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
except Exception:  # noqa: BLE001
    COMMIT = None


def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0


def TH(cfg):
    return "1" if cfg.endswith("_t1") else "4"


def run(cfg, seed, mps, tl):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = TH(cfg); env["ENOMOTO_T_MIP_SEED"] = str(seed)
    t0 = time.time(); peak = 0.0; killed = False
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", TH(cfg)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > tl + 300: pr.kill(); killed = True; break
    out, _ = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "killed" if killed else "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1)


for p, cfg, seed, tl in mine:
    if (p, cfg, seed) in done: continue
    for attempt in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e: err = e; time.sleep(30)  # noqa: BLE001
    else:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": {"status": "fetch_error", "error": str(err)[:200]}, "shard": k}) + "\n")
        continue
    res, peak, wall = run(cfg, seed, mps, tl)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k, "tail": TAIL,
                            "nproc": os.cpu_count(), "commit": COMMIT, "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), peak, flush=True)
