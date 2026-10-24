"""batch_guard.sh の要約: python sum_guard.py <名前>... -> 問題ごとに 状態・目的値・時間・根の下界 (最初の根のカットの終わりの時刻)・再スタートの回数"""
import glob, json, re, sys
P = "noswot mc11 hypothyroid-k1 dcmulti neos-960392 roll3000 physiciansched6-2 h80x6320d binkar10_1".split()
names = sys.argv[1:]
print("問題 | " + " | ".join(names))
for p in P:
    row = []
    for n in names:
        cells = []
        for f in sorted(glob.glob(f"logs/{p}_{n}_[0-9].log")):
            d = None; rc = None; rs = 0
            for line in open(f, errors="replace"):
                m = re.match(r"Root cuts: .*\(([\d.]+)s\)", line)
                if m and rc is None:
                    rc = m.group(1)
                if line.startswith("Restart "):
                    rs += 1
                if line.startswith("{"):
                    d = json.loads(line)
            if d is None:
                cells.append("(途中)"); continue
            st = "opt" if d["status"] == "optimal" else "TL"
            ob = d["objective"]
            cells.append(f"{st} {ob if ob is None else round(ob, 3)} {d['time']:.1f}s 根 {d['root_bound'] if d['root_bound'] is None else round(d['root_bound'], 2)} (カット終 {rc}s, 再 {rs})")
        row.append(" / ".join(cells) if cells else "-")
    print(f"{p} | " + " | ".join(row))
