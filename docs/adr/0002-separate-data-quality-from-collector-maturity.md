# Separate data quality from Collector maturity

AgentMeter records Data Quality independently from Collector Maturity. Authority and integration stability answer different questions: an official preview interface can be authoritative yet experimental, while a stable local parser can produce only locally observed data; keeping both axes prevents the UI and release gates from treating “official” as “stable.”
