set -x
python ours.py cod105 def --log
python ours.py cod105 orbpp --log -- ENOMOTO_MIP_ORBPP=1
python ours.py cod105 sst --log -- ENOMOTO_MIP_SYM_SST=1
python ours.py rail507 def --log
python ours.py rail507 domcol --log -- ENOMOTO_MIP_DOMCOL=1
python ours.py rail507 domcolp --log -- ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DOMCOL_PRED=1
