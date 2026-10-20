until grep -q RESULT fcsep2_2.out 2>/dev/null; do sleep 20; done
M=../.miplib_cache/neos-2978193-inde.mps
for s in 0 1 2; do
python run_highs.py $M 300 4 $s mip_lp_age_limit=32767 | sed "s/^/age32767 s$s /"
python run_highs.py $M 300 4 $s mip_lp_age_limit=2 | sed "s/^/age2 s$s /"
done
python run_highs.py $M 300 4 2 | sed "s/^/def s2 /"
