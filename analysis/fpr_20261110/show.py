import json, sys
for f in sys.argv[1:]:
    for l in open(f):
        r = json.loads(l); res = r["result"]
        acc = [x for x in r["fpr_lines"] if "incumbent" in x or "accepted" in x or "thread:" in x or "found" in x]
        print(f'{r["problem"]:22s} {r["cfg"]:4s} s{r["seed"]} {res.get("status"):10s} t={res.get("time",0):6.1f} first={res.get("first_solution_time") or -1:7.2f} '
              f'fobj={res.get("first_solution_objective")} obj={res.get("objective")} bd={res.get("bound")}')
        for x in acc[:4]:
            print("      ", x[:150])
