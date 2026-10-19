import highspy, sys
h=highspy.Highs(); h.setOptionValue("threads",1); h.setOptionValue("time_limit",float(sys.argv[2])); h.setOptionValue("output_flag",True)
h.readModel(sys.argv[1]); h.run()
