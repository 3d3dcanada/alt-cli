import sys,os
sys.path.insert(0,os.getcwd())
from solution import decode
assert decode('A%20B+C')=='A B+C' and decode('%E2%98%83')=='☃'
assert decode('café%2fok')=='café/ok' and decode('%25')=='%'
for text in ['%','%2','%GG','%+1']:
 try: decode(text)
 except ValueError: pass
 else: raise AssertionError(text)
try: decode('%FF')
except UnicodeDecodeError: pass
else: raise AssertionError('invalid encoded UTF-8')

print('ALT_ORACLE_COMPLETED_7fb5020a83d94e349e319074aee44825')
