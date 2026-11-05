"""logs/<問題><接尾辞>.log (one.py の stderr、ENOMOTO_DEBUG_CROSSOVER=1 で取ったもの) から、根の LP の内点法 + クロスオーバーの
段ごとの要点を 1 問 1 行の表 (Markdown) にする。使い方: python summ.py 接尾辞 問題..."""
import json, re, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
suf, probs = sys.argv[1], sys.argv[2:]
print("| 問題 | 結果 | 内点法 (秒) | 内点法の反復 / 状態 | 相対残差 (主, 双対, ギャップ) / 許容 | 押し出しの後の超基底 | 仕上げ前 主 / 双対実行不能 | warm start の主実行不能 | 仕上げの反復 | 内点法+XO (秒) | 諦めた理由 |")
print("|---|---|---:|---|---|---:|---|---:|---:|---:|---|")
for p in probs:
    f = HERE / "logs" / f"{p}{suf}.log"
    if not f.exists():
        continue
    t = f.read_text(errors="replace")
    # 根の LP の診断の部分 (LP プロービングの競争の分を除く)。書き出しの行がなければ最後の "CROSSOVER ipm" から
    k = t.find("presolved problem written")
    if k < 0:
        k = t.rfind("CROSSOVER ipm status")
        k = t.rfind("\nIPM kkt", 0, k) if k >= 0 else 0
    s = t[max(k, 0):]
    j = re.search(r"^ROOTLP_BOTH (\{.*\})$", s, re.M)
    r = json.loads(j.group(1)) if j else {}
    ipm = re.findall(r"CROSSOVER ipm status=(\w+) iters=(\d+) rel_res=\(([^)]*)\)", s)
    ipm = [x for x in ipm if x[1] != "0"] or ipm
    lo = re.findall(r"leftover superbasics=(\d+)", s)
    q = re.findall(r"basis quality: primal_infeas=(\d+) \(max ([^)]*)\) dual_infeas=(\d+) \(max ([^)]*)\)", s)
    ws = re.findall(r"warm start from a given basis \(cost shifts (\d+), hint agree (\d+) differ (\d+) unknown (\d+), primal infeasible basics (\d+)\)", s)
    mi = re.findall(r"main_loop_iters=(\d+)", s)
    gu = re.findall(r"CROSSOVER (.*giving up.*)", s)
    rel = ", ".join(f"{float(v):.1e}" for v in ipm[-1][2].split(",")) if ipm else "-"
    st = "頂点" if r.get("st_i") == "Some(Optimal)" else "頂点なし"
    print(f"| {p} | {st} | {r.get('t_ipm') or '-'} | {ipm[-1][1] + ' / ' + ipm[-1][0] if ipm else '-'} | {rel} | {lo[-1] if lo else '-'} | "
          f"{q[-1][0] + ' / ' + q[-1][2] + ' (最大 ' + q[-1][3] + ')' if q else '-'} | {ws[-1][4] if ws else '-'} | {mi[-1] if mi else '-'} | {r.get('t_xo', '-')} | {gu[-1][:60] if gu else ''} |")
