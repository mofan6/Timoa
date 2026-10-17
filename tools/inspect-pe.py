"""Read PE metadata/imports without loading or running the inspected executable."""
import argparse
import hashlib
import json
import struct
from pathlib import Path


def inspect(path):
    path = Path(path)
    data = path.read_bytes()
    pe = struct.unpack_from('<I', data, 0x3C)[0]
    if data[:2] != b'MZ' or data[pe:pe+4] != b'PE\0\0':
        raise ValueError(f'Not a PE file: {path}')
    machine, count = struct.unpack_from('<HH', data, pe+4)
    opt_size = struct.unpack_from('<H', data, pe+20)[0]
    opt = pe+24
    magic = struct.unpack_from('<H', data, opt)[0]
    width = 8 if magic == 0x20B else 4
    directory = opt+(112 if width == 8 else 96)
    sections = []
    for i in range(count):
        offset = opt+opt_size+i*40
        virtual_size, rva, raw_size, raw_offset = struct.unpack_from('<IIII', data, offset+8)
        sections.append((rva, max(virtual_size, raw_size), raw_offset))

    def offset_of(rva):
        for start, size, raw in sections:
            if start <= rva < start+size:
                return raw+rva-start
        if rva < struct.unpack_from('<I', data, opt+60)[0]:
            return rva
        raise ValueError(f'Unmapped RVA: {rva:x}')

    def string_at(rva):
        offset = offset_of(rva)
        return data[offset:data.index(b'\0', offset)].decode('ascii')

    import_rva = struct.unpack_from('<I', data, directory+8)[0]
    delay_rva = struct.unpack_from('<I', data, directory+13*8)[0]
    imports = {}
    if import_rva:
        cursor = offset_of(import_rva)
        while any(data[cursor:cursor+20]):
            lookup, _, _, name, address = struct.unpack_from('<IIIII', data, cursor)
            dll = string_at(name).lower()
            target = imports.setdefault(dll, [])
            thunk = offset_of(lookup or address)
            while True:
                value = struct.unpack_from('<Q' if width == 8 else '<I', data, thunk)[0]
                if not value:
                    break
                target.append(f'#{value & 0xffff}' if value & (1 << (width*8-1)) else string_at(value+2))
                thunk += width
            cursor += 20
    for names in imports.values():
        names.sort()
    newer_apis = {
        'GetDpiForWindow', 'GetDpiForSystem', 'GetSystemMetricsForDpi',
        'SystemParametersInfoForDpi', 'AdjustWindowRectExForDpi',
        'SetThreadDpiAwarenessContext', 'SetProcessDpiAwarenessContext',
        'SetProcessDpiAwareness', 'GetSystemTimePreciseAsFileTime',
        'WaitOnAddress', 'WakeByAddressSingle', 'WakeByAddressAll',
        'RoOriginateError', 'RoOriginateErrorW', 'RoGetRestrictedErrorInfo',
        'GetRestrictedErrorInfo', 'SetRestrictedErrorInfo', 'RoCaptureErrorContext',
        'ProcessPrng', 'GetTempPath2W', 'GetTempPath2A', 'SetThreadDescription',
        'GetThreadDescription', 'GetOverlappedResultEx', 'DiscardVirtualMemory',
        'GetSystemCpuSetInformation', 'VirtualAlloc2', 'GetProcessInformation',
    }
    newer_dlls = {'combase.dll', 'bcryptprimitives.dll', 'shcore.dll',
        'ucrtbase.dll', 'vcruntime140.dll', 'msvcp140.dll'}
    blockers = [f'{dll}!{name}' for dll, names in imports.items() for name in names if name in newer_apis]
    blockers.extend(dll for dll in imports if dll in newer_dlls or dll.startswith(('api-ms-win-', 'ext-ms-win-')))
    return {
        'file': path.name,
        'bytes': len(data),
        'sha256': hashlib.sha256(data).hexdigest().upper(),
        'architecture': {0x14C: 'x86', 0x8664: 'x64'}.get(machine, hex(machine)),
        'subsystemVersion': '%d.%d' % struct.unpack_from('<HH', data, opt+48),
        'subsystem': struct.unpack_from('<H', data, opt+68)[0],
        'imports': dict(sorted(imports.items())),
        'hasDelayImports': bool(delay_rva),
        'knownWin7ImportBlockers': sorted(blockers),
        'inspectionScope': 'Static PE metadata and imports only; no execution. The blocker list is not an exhaustive compatibility proof.',
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('files', nargs='+')
    parser.add_argument('--output')
    args = parser.parse_args()
    reports = [inspect(p) for p in args.files]
    text = json.dumps(reports, ensure_ascii=False, indent=2)+'\n'
    if args.output:
        Path(args.output).write_text(text, encoding='utf-8')
        print(json.dumps([{'file': r['file'], 'architecture': r['architecture'],
            'subsystemVersion': r['subsystemVersion'], 'dllCount': len(r['imports']),
            'knownWin7ImportBlockers': r['knownWin7ImportBlockers']} for r in reports]))
    else:
        print(text)
