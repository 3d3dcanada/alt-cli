import sys,os
sys.path.insert(0,os.getcwd())
from solution import utc_timestamp
assert utc_timestamp("2024-01-01T01:00:00+01:00")==1704067200
assert utc_timestamp("2024-01-01T00:00:00Z")==1704067200
try:utc_timestamp("2024-01-01T00:00:00")
except ValueError:pass
else:raise AssertionError()

print('ALT_ORACLE_COMPLETED_5c13bb73415144aa926c7ea499623c76')
