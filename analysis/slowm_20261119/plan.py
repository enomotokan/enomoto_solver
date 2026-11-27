"""P2-SCIP-M の分担表: python plan.py P NP → shards_P.json (MIPLIB の 58 問 × 種 1〜3 × {def, fix, fixis}、60 秒)。
python plan.py Q NQ → shards_Q.json (58 問 × 種 1〜3 × {def, st5}、60 秒)。
python plan.py D ND → shards_D.json (d の前処理が 1 とも既定とも違う M の問題 × 種 1・2 × {isd}、600 秒)。
python plan.py E NE → shards_E.json (遅い側の 5 問 × 種 3〜8 × {def, fixis})、python plan.py M3 NM3 → shards_M3.json (M の 37 問 × 種 3 × {def, fixis})。
python plan.py G NG → shards_G.json (INTSING の `g` 形で消す列が `1` と変わる問題 × 種 1〜3 (2 問は 1〜8) × {def, isg} (同じビルドの対)、600 秒)。
python plan.py H NH → shards_H.json (INTSING の `1gs` 形で消す列が減って 0 本にならない 7 問 × {def, isgs} (同じビルドの対)、600 秒)。
python plan.py L NL → shards_L.json (1go で解けた数が減った supportcase33 種 1〜8・traininstance6 種 9〜16 × {def, isgo}、600 秒)。
python plan.py S NS → shards_S.json (カットの打ち切りが外側の根で働く M の 17 問 × 種 1・2 × {def, st5}、600 秒)。
python plan.py CP NCP → shards_CP.json (58 問 × 種 1〜3 × {def, pe}、60 秒)、python plan.py CM NCM → shards_CM.json (Mittelmann 186 問 × 種 1 × {def, pe}、600 秒)。
python plan.py M NM → shards_M.json (scan_s*.jsonl で前処理が変わった Mittelmann の問題 + 対象 6 問 × 種 1・2 × {def, fixis}、600 秒)。
python plan.py DT NDT → shards_DT.json (58 問 × 種 1〜3 × {def, bonil, def_t1, bonil_t1}、60 秒。1 スレッドの再現性の試作の費用)。
python plan.py PS NPS → shards_PS.json (58 問 × 種 1〜3 × {def, pe, pes}、60 秒)。
python plan.py CM2S NS → shards_CM2S.json (CM2 で 1.25 倍超の差が出た 25 問 × 種 2・3 × {def, pe}、600 秒。種による揺れか確かめる。2026-11-21)。
python plan.py CM2 NCM2 → shards_CM2.json (CM と同じ組を、0.1.10 の既定 (st5・SYM_PPLEX・cwrh0・vbce・ncnr・rhb・cb) の上で回し直す。2026-11-20)。
見積りは analysis/final_20261118 の def の時間 (解けなかった回は上限) + P は 8 秒・M は 30 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
FIN = D.parent / "final_20261118"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
SET, NS = sys.argv[1], int(sys.argv[2])
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = {}
    for f in glob.glob(str(FIN / pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] == "def": T[(r["problem"], r["seed"])] = min(r["result"]["time"], cap) if solved(r) else cap
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
if SET == "P":
    CF = ["def", "fix", "fixis"]; TP = times("res_P_s*.jsonl", 60)
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 3:] + CF[:(i + s) % 3])] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
            lambda p, c, s: TP.get((p, 1), 60) + 8, NS, "shards_P.json", 60)
elif SET == "Q":
    CF = ["def", "st5"]; TP = times("res_P_s*.jsonl", 60)
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
            lambda p, c, s: TP.get((p, 1), 60) + 8, NS, "shards_Q.json", 60)
elif SET == "D":
    # scan_d.jsonl で d の前処理が 1 とも既定とも違う問題 (ほかの問題の d の結果は、前処理が同じ側 (def か fixis) の結果で代える)
    ds = [r["problem"] for l in open(D / "scan_d.jsonl") if l.strip() for r in [json.loads(l)] if r["set"] == "M" and r["d"] != r["one"] and r["d"] != r["none"]]
    TM = times("res_M_s*.jsonl", 600)
    balance({(p, s): [(p, "isd", s)] for p in ds for s in (1, 2)}, lambda p, c, s: TM.get((p, 1), 600) + 30, NS, "shards_D.json", 600)
elif SET == "G":
    # INTSING の `g` 形 (数える一般整数列を消さない) の確かめ: intsing_kinds_1g.jsonl で `1` と消す列の数が変わる 11 問 (どれも M) だけ {def, isg} を回す。
    # ほかの問題は `1` と前処理が同じなので fixis の結果で代える。種 1〜3、E で 8 種まで回した rocI-4-11・traininstance6 は 4〜8 も。600 秒
    import re
    def nel(f):
        return {r["problem"]: sum(int(x) for x in re.findall(r"eliminated (\d+)", " ".join(r["logs"]))) for l in open(D / f) if l.strip() for r in [json.loads(l)]}
    a, b = nel("intsing_kinds.jsonl"), nel("intsing_kinds_1g.jsonl")
    probs = sorted(p for p in a if a[p] != b.get(p))
    tm = {}
    for f in glob.glob(str(D / "res_M*_s*.jsonl")) + glob.glob(str(D / "res_E*.jsonl")):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] == "fixis":
                tm.setdefault(r["problem"], []).append(min(r["result"].get("time") or 600, 600) if solved(r) else 600)
    est = lambda p, c, s: sum(tm.get(p, [600])) / len(tm.get(p, [600])) + 30
    # 既定が d3f31f37c から変わったので (親の指摘)、def も同じビルドで対にして回す。順は問題・種で入れ替える
    CF = ["def", "isg"]
    jobs = {(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(probs)
            for s in ((1, 2, 3, 4, 5, 6, 7, 8) if p in ("rocI-4-11", "traininstance6") else (1, 2, 3))}
    print("G:", probs)
    balance(jobs, est, NS, "shards_G.json", 600)
elif SET == "H":
    # INTSING の `1gs` 形の確かめ: `1` と消す列の数が変わり、しかも 0 本にならない 7 問 (どれも M) で {def, isgs} を同じビルドの対で回す。
    # 0 本になる 14 問 (P 4・M 10) は前処理の行・列が既定と同じ (scan_gs.jsonl) なので既定と同じとみなし、`1` と同じ 15 問は fixis の結果で代える。
    # 種は、両方とも解ける回のある rocI-4-11・piperout-08・piperout-27 が 1〜8、G で全種とも時間切れの問題と rocII-5-11 は 1〜3。600 秒
    probs = ["brazil3", "neos-3656078-kumeu", "piperout-08", "piperout-27", "radiationm18-12-05", "rocI-4-11", "rocII-5-11"]
    tm = {}
    for f in glob.glob(str(D / "res_G_s*.jsonl")) + glob.glob(str(D / "res_M*_s*.jsonl")):
        for l in open(f):
            r = json.loads(l)
            tm.setdefault(r["problem"], []).append(min(r["result"].get("time") or 600, 600) if solved(r) else 600)
    est = lambda p, c, s: sum(tm.get(p, [600])) / len(tm.get(p, [600])) + 30
    CF = ["def", "isgs"]
    jobs = {(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(probs)
            for s in ((1, 2, 3, 4, 5, 6, 7, 8) if p in ("rocI-4-11", "piperout-08", "piperout-27") else (1, 2, 3))}
    balance(jobs, est, NS, "shards_H.json", 600)
elif SET == "L":
    # INTSING の `1go` (= M では 1g と同じ前処理) で解けた数が減った 2 問の運の確かめ: supportcase33 種 1〜8、traininstance6 種 9〜16
    # (種 1〜8 は G で回した)。{def, isgo} を同じビルドの対で、600 秒
    CF = ["def", "isgo"]
    tm = {}
    for f in glob.glob(str(D / "res_G_s*.jsonl")) + glob.glob(str(D / "res_M*_s*.jsonl")) + glob.glob(str(D / "res_E_s*.jsonl")):
        for l in open(f):
            r = json.loads(l)
            tm.setdefault(r["problem"], []).append(min(r["result"].get("time") or 600, 600) if solved(r) else 600)
    est = lambda p, c, s: sum(tm.get(p, [600])) / len(tm.get(p, [600])) + 30
    jobs = {(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, (p, ss) in enumerate([("supportcase33", range(1, 9)), ("traininstance6", range(9, 17))]) for s in ss}
    balance(jobs, est, NS, "shards_L.json", 600)
elif SET == "S":
    # カットの打ち切りの待つ巡数 3 → 5 (st5) の M: scan_st_*.jsonl (Mittelmann 186 問、既定・1 スレッド・種 1 で根のカットのループを見た) で
    # 打ち切り ("LP bound unchanged for k rounds; stop") が外側の根で働いた 17 問 (awhea を含む) × 種 1・2 × {def, st5}、600 秒・4 スレッド。
    # ほかの 169 問は外側の根で打ち切りが働かないので st5 と既定は同じ (4 スレッドでは働く問題が少し違いうる)。見積りは final_20261118 の def
    probs = sorted({r["problem"] for f in glob.glob(str(D / "scan_st_*.jsonl")) for l in open(f) for r in [json.loads(l)] if r["stall"]})
    CF = ["def", "st5"]; TM = times("res_M_s*.jsonl", 600)
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(probs) for s in (1, 2)},
            lambda p, c, s: TM.get((p, s), TM.get((p, 1), 600)) + 30, NS, "shards_S.json", 600)
elif SET in ("DT", "PS"):
    # DT: 1 スレッドの再現性の試作 (DET1T) の費用。4 スレッドの対 {def, bonil} と 1 スレッドの対 {def_t1, bonil_t1}。1 スレッドの見積りは 4 スレッドの 2 倍 (上限 60)
    # PS: pe と σ の上限つきの pe。どちらも同じ (問題, 種) の設定を同じ分担に入れ、順を問題ごとに回す
    CF = ["def", "bonil", "def_t1", "bonil_t1"] if SET == "DT" else ["def", "pe", "pes"]
    TP = times("res_P_s*.jsonl", 60)
    k = len(CF)
    est = lambda p, c, s: (min(2 * TP.get((p, s), TP.get((p, 1), 60)), 60) if c.endswith("_t1") else TP.get((p, s), TP.get((p, 1), 60))) + 8
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % k:] + CF[:(i + s) % k])] for i, p in enumerate(PROBS) for s in (1, 2, 3)}, est, NS, f"shards_{SET}.json", 60)
elif SET == "CM2S":
    # CM2 (種 1) で pe/def が 1.25 倍を超えて離れた問題 (遅い 12・速い 13) を種 2・3 で回し、全種でそろうかを見る。見積りは CM2 の種 1 の時間
    SLOW = ["30n20b8", "enlight_hard", "neos5", "rococoC10-001000", "fiball", "timtab1", "neos-1171448", "supportcase26", "neos-1582420", "gen-ip002", "gen-ip054", "gmu-35-40"]
    FAST = ["fhnw-binpack4-48", "peg-solitaire-a3", "comp07-2idx", "graphdraw-domain", "tbfp-network", "neos-1456979", "CMS750_4", "supportcase33", "neos-2978193-inde", "ns1208400", "neos-3627168-kasai", "irp", "bnatt400"]
    T1 = {}
    for f in glob.glob(str(D / "res_CM2_s*.jsonl")):
        for l in open(f):
            r = json.loads(l); T1.setdefault(r["problem"], []).append(min(r["result"]["time"], 600) if solved(r) else 600)
    CF = ["def", "pe"]
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(SLOW + FAST) for s in (2, 3)},
            lambda p, c, s: sum(T1.get(p, [600])) / len(T1.get(p, [600])) + 30, NS, "shards_CM2S.json", 600)
elif SET in ("CP", "CM", "CM2"):
    # 衝突解析で双対証明を遡る試作 (pe) の広い計測: CP = MIPLIB の 58 問 × 種 1〜3 × {def, pe}、60 秒。CM = Mittelmann 186 問 × 種 1 × {def, pe}、600 秒。
    # 見積りは final_20261118 の def (解けなかった回は上限) + P 8 秒・M 30 秒
    CF = ["def", "pe"]
    if SET == "CP":
        TP = times("res_P_s*.jsonl", 60)
        balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
                lambda p, c, s: TP.get((p, s), TP.get((p, 1), 60)) + 8, NS, "shards_CP.json", 60)
    else:
        TM = times("res_M_s*.jsonl", 600)
        MP = sorted({x for part in json.loads((D.parent / "cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
        balance({(p, 1): [(p, c, 1) for c in (CF[i % 2:] + CF[:i % 2])] for i, p in enumerate(MP)},
                lambda p, c, s: TM.get((p, s), 600) + 30, NS, f"shards_{SET}.json", 600)
elif SET in ("E", "M3"):
    # 運かどうかの確かめ (利用者の判断「遅くなるのが運なら採用」): E = 遅い側に出た 5 問 × 種 3〜8、M3 = M の 37 問 × 種 3。{def, fixis}、600 秒。
    # 見積りは res_M の同じ問題の def・fixis の時間 (2 種の平均) + 30 秒
    CF = ["def", "fixis"]
    tm = {}
    for f in glob.glob(str(D / "res_M_s*.jsonl")):
        for l in open(f):
            r = json.loads(l); tm.setdefault((r["problem"], r["cfg"]), []).append(min(r["result"].get("time") or 600, 600) if solved(r) else 600)
    est = lambda p, c, s: sum(tm.get((p, c), [600])) / len(tm.get((p, c), [600])) + 30
    if SET == "E":
        probs, seeds = ["traininstance6", "supportcase33", "30n20b8", "rocI-4-11", "neos-1171448"], range(3, 9)
    else:
        probs, seeds = sorted({t[0] for s in json.loads((D / "shards_M.json").read_text())["shards"] for t in s}), [3]
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(probs) for s in seeds}, est, NS, f"shards_{SET}.json", 600)
else:
    CF = ["def", "fixis"]; TM = times("res_M_s*.jsonl", 600)
    ch = {r["problem"] for f in glob.glob(str(D / "scan_s*.jsonl")) for l in open(f) if l.strip() for r in [json.loads(l)] if r.get("changed")}
    ch |= {"neos-3381206-awhea", "neos-1171448", "neos-1445765", "cod105", "decomp2", "irp"}
    balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 2:] + CF[:(i + s) % 2])] for i, p in enumerate(sorted(ch)) for s in (1, 2)},
            lambda p, c, s: TM.get((p, 1), 600) + 30, NS, "shards_M.json", 600)
