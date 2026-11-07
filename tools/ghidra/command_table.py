#!/usr/bin/env python3
"""Lists AMIPRO.EXE's menu-command table: each command ID and the far
address (selector:offset) of its handler, read from the unpacked file
(docs/research/DECOMPILATION.md). The table sits in DGROUP (segment 209)
at offset 0x46: entries of (command, offset, segment), the segment filled
by a relocation, ending at command 0 (dispatcher 1010:0b07). About 320 of
its handlers are reached only through it, so Ghidra's auto-analysis does
not find them; DecompAt.java decompiles them from this list.

Usage: tools/ghidra/command_table.py UNPACKED.EXE [ID ...]   (IDs in hex)
Prints "SSSS:OOOO cmd 0xID" lines, the input DecompAt.java takes.
"""
import struct
import sys

DGROUP = 209
TABLE = 0x46


def main():
    data = open(sys.argv[1], "rb").read()
    wanted = {int(value, 16) for value in sys.argv[2:]}
    ne = struct.unpack_from("<I", data, 0x3C)[0]
    segtab = ne + struct.unpack_from("<H", data, ne + 0x22)[0]
    shift = struct.unpack_from("<H", data, ne + 0x32)[0]
    offset, length, flags, _ = struct.unpack_from("<HHHH", data, segtab + 8 * (DGROUP - 1))
    start, length = offset << shift, length or 0x10000
    # Internal-reference relocations that fill a selector: location -> segment.
    selectors = {}
    if flags & 0x100:
        pos = start + length
        count = struct.unpack_from("<H", data, pos)[0]
        for index in range(count):
            source, kind, location, segment, _ = struct.unpack_from(
                "<BBHHH", data, pos + 2 + 8 * index)
            if source != 2 or kind & 3 != 0:
                continue
            # A chain of locations unless additive (bit 2), as in standard NE.
            while location < length:
                selectors[location] = segment & 0xFF
                if kind & 4:
                    break
                location = struct.unpack_from("<H", data, start + location)[0]
    pos = TABLE
    while True:
        command, target, _ = struct.unpack_from("<HHH", data, start + pos)
        if command == 0:
            break
        segment = selectors.get(pos + 4)
        if segment and (not wanted or command in wanted):
            print("%04x:%04x cmd 0x%x" % (0x1000 + 8 * (segment - 1), target, command))
        pos += 6


if __name__ == "__main__":
    main()
