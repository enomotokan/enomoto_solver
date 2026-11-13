"""shards_L.json を作る。分担 = (問題, 種) ごとに cand1114・cand1113・+X 6 形 (600 秒)。ビルドは 54a057b8 (cand1114 の P・M と同じ)。
分担 0・1: fiball 種 1・2、2・3: ran14x18-disj-8 種 1・2、4・5: csched008 種 1・2、6: neos-2978193-inde 種 1・2 (軽い)。
衝突の直し (f1c14c30・dfb8186d) の影響は、直した後のビルドで cand1114・cand1113 を回して別に見る (分担 F、shards_F.json)。"""
import json
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
FORMS = ["cand1114", "cand1113"] + [c for c in CFGS if c.startswith("+")]
sh = []
for p in ("fiball", "ran14x18-disj-8", "csched008"):
    for s in (1, 2):
        sh.append([[p, c, s, 600] for c in FORMS])
sh.append([["neos-2978193-inde", c, s, 600] for s in (1, 2) for c in FORMS])
(D / "shards_L.json").write_text(json.dumps({"shards": sh}))
F = [[[p, c, s, 600] for p in ("fiball", "neos-2978193-inde") for s in (1, 2) for c in ("cand1114", "cand1113")]]
(D / "shards_F.json").write_text(json.dumps({"shards": F}))
print(len(sh), sum(map(len, sh)), len(F[0]))
