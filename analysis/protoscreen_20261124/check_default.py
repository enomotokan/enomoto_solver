"""既定の動きが変わっていないかの確かめ: python check_default.py [環境変数=値 ...]
入れた当方 (pip install -e . した分岐の先端) で、学習・計測と重ならない P の 6 問を 1 スレッド・種 1・上限 200 秒 (決定的) で解き、状態・ノード数・目的値を
基準と比べる。基準は analysis/losses_20261123/res_pgocheck.jsonl の lto_v3 の回 (40be041e4 のソース、今回のふるい分けの試作の多くが入る前。
作り方 (LTO・target-cpu) は道筋を変えないことを buildopt・losses 3.1 節で確かめてある)。
環境変数を渡すと、その設定で解いて基準と比べる (試作を入れたときにどこから道筋が変わるかを見る用)。
**設定は必ず引数 (`ENOMOTO_…=値`) で渡す。** 既定の確かめなので、呼んだ側の環境にある `ENOMOTO_` で始まる環境変数はすべて外して解く
(`ENOMOTO_LU_REFAC_BAL=0 python check_default.py` は効かない。`python check_default.py ENOMOTO_LU_REFAC_BAL=0` と書く)。
外した環境変数は、始めに名前と値を標準エラーに出す (2026-11-27、黙って無視しないように)。
2026-11-25: POLISH_STRICT=2 + POLISH_WARM=d の既定化で mik-250-20-75-4 のノード数が変わった (6333 → 6261、目的値は同じ) ので、
基準を `ref_default.json` (既定化の後のソースで、この台本で解いた 6 問) に取り替えた。前の基準は `--old` で使う
(`ENOMOTO_MIP_POLISH_STRICT=0 ENOMOTO_MIP_POLISH_WARM=0` を付けると前の基準と全部一致する)。
2026-11-25 (2 回目): `ENOMOTO_MIP_GLOBALFIX=abc` の既定化で mik-250-20-75-4 のノード数が変わった (6261 → 6157、目的値は同じ。2 回解いて同じ) ので、
`ref_default.json` を書き直した。`ENOMOTO_MIP_GLOBALFIX=0` を引数で渡すと前の基準 (6261) と全部一致する (analysis/globalfix_20261124 5 節)。
2026-11-25 (3 回目): `ENOMOTO_MIP_OBJ_PROP=1` + `ENOMOTO_MIP_OBJ_PROP_INT=1` の既定化で mik-250-20-75-4 (6157 → 5856)・markshare_4_0 (236024 → 272319) の
ノード数が変わった (目的値は同じ。2 回解いて同じ) ので、`ref_default.json` を書き直した。`ENOMOTO_MIP_OBJ_PROP=0` を引数で渡すと 2 回目の基準と全部一致する
(analysis/objprop_20261124 6 節)。
2026-11-26: `ENOMOTO_MIP_POLISH_FINAL` (研磨は返す前の 1 回だけ = `POLISH_WARM=e` + `ROOTPAR_PH=gj`) の既定化で mik-250-20-75-4 のノード数が変わった
(5856 → 6277、目的値は同じ。2 回解いて同じ) ので、`ref_default.json` を書き直した。`ENOMOTO_MIP_POLISH_FINAL=0` を引数で渡すと前の基準と全部一致する
(analysis/rootpar_ph_20261126 の 5.11 節)。
2026-11-27: `ENOMOTO_MIP_PATHH_SLACK=1` の既定化で exp-1-500-5-5 (7 → 5)・qnet1 (3 → 4)・mik-250-20-75-4 (6277 → 3703) のノード数が変わった
(目的値は同じ。2 回解いて同じ) ので、`ref_default.json` を書き直した。`ENOMOTO_MIP_PATHH_SLACK=0` を引数で渡すと前の基準と全部一致する
(analysis/rootbound_20261126 5 節)。
2026-11-27 (2 回目): `ENOMOTO_MIP_ALNS=1b` (alnsc の形) の既定化で mik-250-20-75-4 (3703 → 3657)・markshare_4_0 (272319 → 248404) のノード数が変わった
(目的値は同じ。2 回解いて同じ) ので、`ref_default.json` を書き直した。`ENOMOTO_MIP_ALNS=0` を引数で渡すと前の基準と全部一致する
(analysis/alns_20261126 10 節)。
2026-11-27 (3 回目): `ENOMOTO_LU_REFAC_BAL=k` (再分解を分解と求解の手間の釣り合いで決める) の既定化で exp-1-500-5-5 (5 → 9)・mik-250-20-75-4 (3657 → 3937)・
markshare_4_0 (248404 → 240417) のノード数が変わった (目的値は同じ。2 回解いて同じ) ので、`ref_default.json` を書き直した。`ENOMOTO_LU_REFAC_BAL=0` を引数で渡すと
前の基準と全部一致する (analysis/lpgap_20261126 9.10 節)。
2026-11-27 (4 回目): `ENOMOTO_MIP_PRR_TIGHT=o` の既定化では 6 問とも基準と一致した (道筋が変わらない) ので、`ref_default.json` は書き直していない
(analysis/rootbound_20261126 6 節)。
`--save` で今の結果を `ref_default.json` に書く (既定の道筋を変えたときだけ。理由をここに書き足す)。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OLD = "--old" in sys.argv
SAVE = "--save" in sys.argv
REF = {}
for l in open(D.parent / "losses_20261123" / "res_pgocheck.jsonl"):
    r = json.loads(l)
    if r["ver"] == "lto_v3" and r["rep"] == 0:
        REF[r["problem"]] = r["result"]
if not OLD and (D / "ref_default.json").exists():
    REF = {p: v for p, v in json.loads((D / "ref_default.json").read_text())["results"].items()}
extra = dict(a.split("=", 1) for a in sys.argv[1:] if not a.startswith("--"))
dropped = sorted(k for k in os.environ if k.startswith("ENOMOTO_"))
if dropped:
    print("check_default: 環境変数 " + " ".join(f"{k}={os.environ[k]}" for k in dropped) + " は無視した。設定は引数で渡すこと"
          + " (例: python check_default.py ENOMOTO_LU_REFAC_BAL=0)", file=sys.stderr, flush=True)
got = {}
bad = []
for p, ref in REF.items():
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", **extra})
    out = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "200", "--threads", "1"],
                         cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True).stdout
    res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    got[p] = {k: res[k] for k in ("status", "nodes", "objective")}
    ok = res["status"] == ref["status"] and res["nodes"] == ref["nodes"] and abs((res["objective"] or 0) - (ref["objective"] or 0)) <= 1e-6 * max(1, abs(ref["objective"] or 0))
    print(f"{p}: {res['status']} ノード {res['nodes']} 目的値 {res['objective']} ({res['time']:.1f} 秒) / 基準 {ref['status']} {ref['nodes']} {ref['objective']} → {'一致' if ok else '違う'}", flush=True)
    if not ok: bad.append(p)
print("全部一致 (既定の道筋は基準と同じ)" if not bad else f"違う問題: {bad}")
if SAVE:
    (D / "ref_default.json").write_text(json.dumps({"note": "既定の 1 スレッド・種 1 の結果 (最後の書き直しは check_default.py の説明の最後の日付)。check_default.py --save で作った", "results": got}, ensure_ascii=False, indent=1) + "\n")
    print("ref_default.json に書いた")
