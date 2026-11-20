"""INTSING の d 形の見積り: scan_d.jsonl で d の前処理が既定と同じ問題は def の行、1 と同じ問題は fixis の行を「dmix」として写し、
d が両方と違う問題は res_D (isd) の行を使う (なければその問題を除く)。使い方: python dmix.py SET 出力.jsonl 入力 jsonl ... (SET は P か M)"""
import json, sys
SET, out = sys.argv[1], sys.argv[2]
sd = {r["problem"]: r for l in open("scan_d.jsonl") if l.strip() for r in [json.loads(l)]}
rows = [json.loads(l) for f in sys.argv[3:] for l in open(f) if l.strip()]
with open(out, "w") as fo:
    for r in rows:
        s = sd.get(r["problem"])
        if r["cfg"] == "def":
            fo.write(json.dumps(r) + "\n")
        kind = None if s is None else ("none" if s["d"] == s["none"] else "one" if s["d"] == s["one"] else "diff")
        src = {"none": "def", "one": "fixis", "diff": "isd"}.get(kind, "def")
        if r["cfg"] == src:
            fo.write(json.dumps(dict(r, cfg="dmix")) + "\n")
