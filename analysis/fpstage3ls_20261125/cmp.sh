#!/bin/bash
# 第 3 段の試作の比べ (1 スレッド、種 1〜3)。res_one.jsonl に同じ (問題, 札, 種, 1 スレッド) の行があれば飛ばす。
# 使い方 (どこからでも): bash analysis/fpstage3ls_20261125/cmp.sh 札 [札 ...]
#   札: adm (HEUR_ADAPT=1m、比べの基準)・lsa (adm + FP_STAGE3_LS=a)・lsb (adm + FP_STAGE3_LS=b)。どれも LMIP_SCORE_TICKS=1 を入れる
# SET の既定は第 3 段が効いた問題 (heuradapt の 5 問)、SET=h は heuradapt の害の出た 8 問、SET=x は swath3・neos-1582420・app1-1
cd "$(dirname "$0")/../.."
D=analysis/fpstage3ls_20261125
B="ENOMOTO_MIP_HEUR_ADAPT=1m ENOMOTO_MIP_LMIP_SCORE_TICKS=1 ENOMOTO_MIP_FP_STAGE3_LS_DEBUG=1"
for tag in "$@"; do
  case $tag in
    adm) E="$B";;
    lsa) E="$B ENOMOTO_MIP_FP_STAGE3_LS=a";;
    lsb) E="$B ENOMOTO_MIP_FP_STAGE3_LS=b";;
    *) echo "unknown tag $tag"; exit 1;;
  esac
  PS="10teams:120 neos-1171448:120 CMS750_4:200 noswot:60 pg:60"
  [ "${SET:-}" = h ] && PS="eil33-2:60 swath1:60 mas76:60 markshare_4_0:60 blend2:60 misc07:60 pk1:60 roll3000:60"
  [ "${SET:-}" = x ] && PS="swath3:120 neos-1582420:120 app1-1:120"
  for pt in $PS; do
    p=${pt%%:*}; tl=${pt##*:}
    for s in ${SEEDS:-1 2 3}; do
      grep -q "\"problem\": \"$p\", \"tag\": \"$tag\", \"seed\": $s, \"threads\": 1," $D/res_one.jsonl 2>/dev/null && continue
      bash $D/one.sh $p $s $tl 1 $tag $E
    done
  done
done
