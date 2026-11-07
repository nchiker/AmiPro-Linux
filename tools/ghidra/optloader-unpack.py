#!/usr/bin/env python3
"""Rebuilds an OPTLOADER-packed self-loading NE executable (Ami Pro 3.1's
AMIPRO.EXE) as an ordinary NE file that Ghidra's NE loader can import.

OPTLOADER (SLR Systems, 1993) compresses every segment after the first with
a bit-oriented LZ scheme and writes its own relocation format after each
stream; the first segment holds the loader, whose tail is compressed the
same way. This tool reimplements the decompression and converts the
relocation groups to standard NE relocation records
(docs/research/DECOMPILATION.md). It reads and writes files outside the
repository only; no Ami Pro bytes are kept here.

Usage: tools/ghidra/optloader-unpack.py IN.EXE OUT.EXE
"""
import struct
import sys

class Bits:
    def __init__(self, data, si):
        self.d = data
        self.bp = data[si] | data[si + 1] << 8
        self.si = si + 2
        self.dx = 16

    def bit(self):
        b = (self.bp >> 15) & 1
        self.bp = (self.bp << 1) & 0xFFFF
        self.dx -= 1
        if self.dx == 0:
            self.bp = self.d[self.si] | self.d[self.si + 1] << 8
            self.si += 2
            self.dx = 16
        return b

    def bits(self, n):
        v = 0
        for _ in range(n):
            v = v * 2 + self.bit()
        return v

    def byte(self):
        b = self.d[self.si]
        self.si += 1
        return b


def unpack(data, start, limit=1 << 20, prefix=b""):
    s = Bits(data, start)
    out = bytearray(prefix)
    while len(out) < limit:
        if s.bit():                      # 0x62: literal
            out.append(s.byte())
            continue
        if not s.bit():                  # 0x69 -> 0x70
            cx = 2 + s.bit()             # 0x72..0x78
            if cx == 2:                  # 0x7a: short offset, bh = 0
                high = 0
            else:
                high = offset_high(s)
        else:                            # 0xcb
            if not s.bit():
                cx = 4 + s.bit()         # 0x72 with cx = 2
            elif not s.bit():
                cx = 6 + s.bit()         # 0x72 with cx = 3
            elif not s.bit():
                cx = 8 + s.bits(2)       # 0x138, bx = 0x0802
            elif not s.bit():
                cx = 12 + s.bits(3)      # bx = 0x0c03
            else:
                cl = s.byte()            # 0xeb
                if cl == 0x81:
                    continue             # marker: no copy
                if cl > 0x81:
                    break                # end
                cx = cl
            high = offset_high(s)
        low = s.byte()                   # 0x91
        distance = (high << 8 | low) + 1
        src = len(out) - distance
        if src < 0:
            raise ValueError(f'offset before start at output {len(out)}')
        for _ in range(cx):
            out.append(out[src])
            src += 1
    return bytes(out), s.si


def offset_high(s):                      # 0x7f..0x8e
    if s.bit():                          # 0xfd
        if not s.bit():
            if not s.bit():
                return 4 + s.bits(2)     # cx = 0x0402
            return 8 + s.bits(3)         # 0x0803
        if not s.bit():
            return 16 + s.bits(4)        # 0x1004
        if not s.bit():
            return 32 + s.bits(4)        # 0x2004
        if not s.bit():
            return 48 + s.bits(4)        # 0x3004
        return 64 + s.bits(6)            # 0x4006
    if s.bit():                          # 0xae
        if not s.bit():
            return 1
        return 2 + s.bits(1)             # 0x0201
    return 0


ADDR = {0: 0, 1: 2, 2: 3, 3: 5}  # low byte, selector, far pointer, offset


def convert_relocs(data, si, total):
    """OPTLOADER groups at data[si:] -> list of 8-byte NE records."""
    records = []
    while total > 0:
        kind_byte, count = data[si], data[si + 1]
        si += 2
        total -= count
        if total < 0:
            raise ValueError('relocation count overrun')
        if kind_byte == 0xF0:
            for _ in range(count):
                seg = data[si]
                head = struct.unpack_from('<H', data, si + 1)[0]
                si += 3
                records.append(struct.pack('<BBHBBH', 2, 0, head, seg, 0, 0))
            continue
        addr = ADDR[kind_byte & 3]
        additive = 4 if kind_byte & 4 else 0
        kind = (kind_byte >> 3) & 3
        if kind_byte & 0xE0:
            raise ValueError(f'unknown relocation type {kind_byte:#x}')
        if kind == 0:
            seg = data[si]
            si += 1
            for _ in range(count):
                at, target = struct.unpack_from('<HH', data, si)
                si += 4
                records.append(struct.pack('<BBHBBH', addr, 0 | additive, at, seg, 0, target))
        elif kind in (1, 2):
            module = struct.unpack_from('<H', data, si)[0]
            si += 2
            for _ in range(count):
                at, target = struct.unpack_from('<HH', data, si)
                si += 4
                records.append(struct.pack('<BBHHH', addr, kind | additive, at, module, target))
        else:
            fixup = data[si]
            si += 2
            for _ in range(count):
                at = struct.unpack_from('<H', data, si)[0]
                si += 2
                records.append(struct.pack('<BBHHH', addr, 3 | additive, at, fixup, 0))
    return records, si


def main(src, dst):
    d = open(src, 'rb').read()
    ne = struct.unpack_from('<I', d, 0x3c)[0]
    H = lambda o: struct.unpack_from('<H', d, ne + o)[0]
    nseg, segtab, align = H(0x1c), ne + H(0x22), H(0x32)
    out = bytearray(d)
    # New segments go after the original bytes (resources and name tables
    # keep their offsets) in 512-byte units, so offsets reach 32 MB.
    new_align = 9
    unit = 1 << new_align
    stats = {'segments': 0, 'relocs': 0}
    for i in range(nseg):
        off, length, flags, minalloc = struct.unpack_from('<HHHH', d, segtab + 8 * i)
        pos = off << align
        length = length or 0x10000
        raw = d[pos:pos + length]
        if i == 0:
            # The loader segment: its own tail unpacked in place (0x1e7,
            # length at 0x2e), its standard relocations kept.
            n = struct.unpack_from('<H', raw, 0x2e)[0]
            body, _ = unpack(raw[0x1e7:0x1e7 + n] + b'\0' * 4, 0, prefix=raw[:0x1e7])
            records = []
            if flags & 0x100:
                count = struct.unpack_from('<H', d, pos + length)[0]
                records = [d[pos + length + 2 + 8 * r: pos + length + 10 + 8 * r] for r in range(count)]
        else:
            total = struct.unpack_from('<H', raw, 0)[0]
            body, end = unpack(raw + b"\0" * 4, 2)
            records, end = convert_relocs(raw, end, total)
            if end != len(raw):
                print(f'segment {i + 1}: {len(raw) - end} bytes after the relocations')
            if minalloc and len(body) != minalloc and len(body) > minalloc:
                print(f'segment {i + 1}: unpacked {len(body)} > minalloc {minalloc}')
        while len(out) % unit:
            out.append(0)
        new_off = len(out) >> new_align
        out += body
        new_flags = flags & ~0x100
        if records:
            out += struct.pack('<H', len(records)) + b''.join(records)
            new_flags |= 0x100
        struct.pack_into('<HHHH', out, segtab + 8 * i, new_off, len(body) & 0xFFFF, new_flags, minalloc)
        stats['segments'] += 1
        stats['relocs'] += len(records)
    flags = H(0x0c) & ~0x0800
    struct.pack_into('<H', out, ne + 0x0c, flags)
    struct.pack_into('<H', out, ne + 0x32, new_align)
    open(dst, 'wb').write(out)
    print(stats, 'bytes', len(out))


main(sys.argv[1], sys.argv[2])
