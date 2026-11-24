#!/bin/bash
# 対象の 5 問 (1 スレッド、種 1〜3) を設定ごとに回す。res_one.jsonl に同じ (問題, 札, 種, 1 スレッド) の行があれば飛ばす。
# 使い方 (どこからでも): bash analysis/heuradapt_20261124/cmp.sh 札 [札 ...]  (札: def・fl・fps・ad)
# SET=h なら、protoscreen (P の 1 スレッド) で HEUR1T=fl の害が出た問題 (60 秒) を回す
cd "$(dirname "$0")/../.."
D=analysis/heuradapt_20261124
for tag in "$@"; do
  case $tag in
    def) E="";;
    fl) E="ENOMOTO_MIP_HEUR1T=fl";;
    fps) E="ENOMOTO_MIP_FP_STAGE3=1 ENOMOTO_MIP_POOR_NOINC=1 ENOMOTO_MIP_P2RIT=a ENOMOTO_MIP_FJSYNC_LABEL=1";;
    ad) E="ENOMOTO_MIP_HEUR_ADAPT=1 ENOMOTO_MIP_HEUR_ADAPT_DEBUG=1";;
    *) echo "unknown tag $tag"; exit 1;;
  esac
  PS="10teams:120 neos-1171448:120 CMS750_4:200 noswot:60 pg:60"
  [ "${SET:-}" = h ] && PS="eil33-2:60 swath1:60 mas76:60 markshare_4_0:60 blend2:60 misc07:60 pk1:60 roll3000:60"
  for pt in $PS; do
    p=${pt%%:*}; tl=${pt##*:}
    for s in 1 2 3; do
      grep -q "\"problem\": \"$p\", \"tag\": \"$tag\", \"seed\": $s, \"threads\": 1," $D/res_one.jsonl 2>/dev/null && continue
      bash $D/one.sh $p $s $tl 1 $tag $E
    done
  done
done
