import sys,os
sys.path.insert(0,os.getcwd())
from solution import lines
data='☃\r\n\ncedar\nlast'.encode()
assert list(lines([bytes([b]) for b in data]))==['☃','','cedar','last']
assert list(lines([]))==[] and list(lines([b'\n']))==['']
assert list(lines([b'x\r']))==['x\r']
try: list(lines([b'\xe2',b'\x28']))
except UnicodeDecodeError: pass
else: raise AssertionError('invalid UTF-8')

print('ALT_ORACLE_COMPLETED_23e5344bc75f421d9289bbb869e8d32c')
