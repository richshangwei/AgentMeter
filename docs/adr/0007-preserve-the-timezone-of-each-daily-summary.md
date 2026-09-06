# Preserve the timezone of each Daily Summary

Every Daily Summary records the timezone that defined its day bucket. Changing the configured timezone applies to new summaries and may rebuild periods whose raw events still exist, while older summaries remain in their original timezone and are labelled accordingly; AgentMeter never claims to have recalculated history after the necessary raw data has expired.
