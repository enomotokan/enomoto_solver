"""動いている求解のプロセスを 3 秒ごとに見て、RSS・スレッドの名前ごとの数・大きい無名の写像 (smaps の Rss ≥ 32 MB) を記録する (2026-11-03)。
使い方: python watch_proc.py <pid> <出力 .jsonl>"""
import json, os, re, sys, time
pid, out = int(sys.argv[1]), sys.argv[2]
t0 = time.time()
with open(out, "w") as f:
    while os.path.exists(f"/proc/{pid}"):
        try:
            names = {}
            for t in os.listdir(f"/proc/{pid}/task"):
                n = re.sub(r"\d+$", "", open(f"/proc/{pid}/task/{t}/comm").read().strip())
                names[n] = names.get(n, 0) + 1
            big, cur, tot_anon = [], None, 0
            for l in open(f"/proc/{pid}/smaps"):
                if re.match(r"^[0-9a-f]+-[0-9a-f]+ ", l):
                    a, b = (int(x, 16) for x in l.split()[0].split("-"))
                    cur = [(b - a) >> 20, l.split()[5] if len(l.split()) > 5 else "", 0]
                elif l.startswith("Rss:") and cur is not None:
                    cur[2] = int(l.split()[1]) >> 10
                    if not cur[1]:
                        tot_anon += cur[2]
                    big.append(cur)
            rss = [int(l.split()[1]) >> 10 for l in open(f"/proc/{pid}/status") if l.startswith("VmRSS")][0]
            # 分類: mimalloc の 1 GB 単位の領域 (無名・≥ 1000 MB)・glibc の主ヒープ ([heap])・glibc のスレッドの領域 (無名・60〜64 MB)・
            # そのほかの無名 (大きいものは個別に)・ファイル
            cat = {"mi_1g": 0, "heap": 0, "arena64": 0, "arena64_n": 0, "anon_other": 0, "file": 0}
            for v, nm, rs in big:
                if nm == "[heap]": cat["heap"] += rs
                elif nm: cat["file"] += rs
                elif v >= 1000: cat["mi_1g"] += rs
                elif 60 <= v <= 64: cat["arena64"] += rs; cat["arena64_n"] += 1
                else: cat["anon_other"] += rs
            f.write(json.dumps({"t": round(time.time() - t0, 1), "rss": rss, "anon": tot_anon, "threads": names, "cat": cat,
                                "big_other": sorted([b for b in big if not b[1] and not (60 <= b[0] <= 64) and b[2] >= 16], key=lambda x: -x[2])[:20]}) + "\n"); f.flush()
        except Exception:  # noqa: BLE001
            pass
        time.sleep(3)
