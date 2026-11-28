"""分担表: python plan.py [分担の数 (既定 4)] → shards.json。6 問 × 種 1〜3 × {def, heurfp} × {1, 4 スレッド}、600 秒 = 72 回。
問題: base1 の 1 スレッドで遅い向きの mc11・pk1 と、heurfp の狙いの 30n20b8・CMS750_4・bnatt400・neos-1171448。
同じ (問題, 種, スレッド) の def と heurfp は同じ分担で続けて回す。見積りは 1 回 600 秒 (上限) として、組を順に回り番で分ける。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
NS = int(sys.argv[1]) if len(sys.argv) > 1 else 4
TL = 600
PROBS = ["CMS750_4", "bnatt400", "neos-1171448", "30n20b8", "mc11", "pk1"]
groups = [(p, th, s) for p in PROBS for th in ("t1", "t4") for s in (1, 2, 3)]
shards = [[] for _ in range(NS)]
for i, (p, th, s) in enumerate(groups):
    cf = [f"def_{th}", f"heurfp_{th}"] if i % 2 == 0 else [f"heurfp_{th}", f"def_{th}"]
    shards[i % NS] += [[p, c, s, TL] for c in cf]
(D / "shards.json").write_text(json.dumps({"shards": shards}, indent=0))
print(f"shards.json: {len(groups)} 組 = {sum(len(x) for x in shards)} 回、{NS} 分担 (各 {[len(x) for x in shards]} 回、最長 {max(len(x) for x in shards) * TL / 3600:.1f} 時間)")
