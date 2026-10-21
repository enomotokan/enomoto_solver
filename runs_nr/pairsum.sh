#!/bin/bash
# RESTPAIR をスレッドについて合計して表にする。使い方: pairsum.sh <err ファイル...>
cat "$@" | grep RESTPAIR | awk '{k=$5" bin"$7; for (i=8;i<=NF;i+=3){name=$i; n=$(i+1); a=$(i+2); N[k","name]+=n; S[k","name]+=n*a; names[name]=1}; keys[k]=1}
END {printf "%-16s", "kind"; split("R_saved R_exact R_unit R_nb K_unit K_warm K_exact", nm, " "); for (j=1;j<=7;j++) printf " %9s", nm[j]; printf "\n";
for (k in keys) {printf "%-16s", k; for (j=1;j<=7;j++){x=k","nm[j]; if (N[x]>0) printf " %5.0f(%d)", S[x]/N[x], N[x]; else printf " %9s","-"}; printf "\n"}}' | sort
