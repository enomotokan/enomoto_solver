"""RSTCRIT の分担 K を回す (analysis/rstcrit_20261111.md)。shards.json の K 番目の (設定, 問題, 時間, 種) を順に run.py で 1 回ずつ
(4 スレッド) 解き、res_sK.jsonl に足す。途中から再開できる (済んだものは飛ばす)。ソルバーは同時に 1 つ。
使い方 (リポジトリの根で): sh analysis/rstcrit_20261111/build.sh && python analysis/rstcrit_20261111/run_share.py K"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
k = int(sys.argv[1])
mine = json.loads((D / "shards.json").read_text())["shards"][k]
out = f"res_s{k}.jsonl"
done = set()
if (D / out).exists():
    for l in (D / out).read_text().splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["cfg"], r["problem"], int(r["tl"]), r["rep"]))
for c, p, tl, s in mine:
    if (c, p, int(tl), s) in done:
        continue
    subprocess.run([sys.executable, str(D / "run.py"), c, p, str(tl), str(s), out], check=False)
print("SHARE DONE", k, flush=True)
