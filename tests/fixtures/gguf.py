"""Minimal valid container for import UI tests; one float, no actual model."""
import struct

def fixture():
    def string(value):
        raw = value.encode()
        return struct.pack('<Q', len(raw)) + raw
    data = b'GGUF' + struct.pack('<IQQ', 3, 1, 1)
    data += string('general.architecture') + struct.pack('<I', 8) + string('fixture-no-model')
    data += string('fixture.scalar') + struct.pack('<IQIQ', 1, 1, 0, 0)
    return data + b'\0' * (-len(data) % 32) + struct.pack('<f', 0)
