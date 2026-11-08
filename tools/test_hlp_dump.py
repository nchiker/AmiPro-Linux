#!/usr/bin/env python3
"""Unit tests for hlp_dump.py on a synthetic mini WinHelp 3.1 file built here
(no proprietary data).  Run: python3 -I tools/test_hlp_dump.py"""
import os
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import hlp_dump as H  # noqa: E402


def lz_literal(data):
    """Valid LZ77 stream using only literals, plus one back reference at the
    end when the data ends with a repeat (exercises the match path)."""
    out = bytearray()
    for i in range(0, len(data), 8):
        chunk = data[i:i + 8]
        out.append(0)
        out += chunk
    return bytes(out)


def lz_with_match(prefix, repeat_len, dist):
    """prefix as literals, then one match copying repeat_len bytes from dist back."""
    out = bytearray()
    lits = prefix
    i = 0
    while i < len(lits):
        chunk = lits[i:i + 8]
        if len(chunk) == 8 or i + 8 >= len(lits):
            pass
        out.append(0)
        out += chunk
        i += 8
    out.append(0x01)  # one match, first bit set
    w = ((repeat_len - 3) << 12) | (dist - 1)
    out += struct.pack("<H", w)
    return bytes(out)


def btree(structure, entries, keyfn):
    page = bytearray()
    for k, v in entries:
        page += keyfn(k) + v
    psize = 2048
    leaf = struct.pack("<HHHH", psize - 8 - len(page), len(entries), 0xFFFF, 0xFFFF) + page
    leaf = leaf.ljust(psize, b"\0")
    hdr = struct.pack("<HHH", 0x293B, 0, psize) + structure.ljust(16, b"\0") + \
        struct.pack("<HHHHHHI", 0, 0, 0, 0xFFFF, 1, 1, len(entries))
    return hdr + leaf


def csl_enc(v):
    return struct.pack("<H", ((v + 0x4000) << 1)) if -0x4000 <= v < 0x4000 else None


def build_hlp():
    # phrases: "Hello" and "world"; table raw, data LZ77 (literals)
    phr = [b"Hello", b"world"]
    table = struct.pack("<3H", 6, 11, 16)
    pdata = b"".join(phr)
    body = table + pdata
    comp = lz_literal(pdata)
    phrases_file = struct.pack("<HHI", 2, 0x100, len(body)) + table + comp

    title = b"Mini Topic"
    # display record: strings "", phrase0+space & phrase1 & "!", "" for the
    # commands font-change, end-of-paragraph, end (0x80 03 00, 0x82, 0xFF)
    ld2 = b"\0Hello world!\0\0"
    disp_raw = b"\0" + bytes([1, 1]) + bytes([1, 2]) + b"!\0\0"
    ld1 = csl_enc(0) + bytes([len(ld2) << 1]) + b"\0\x80\0\0" + b"\0\0" + b"\x80\x03\x00\x82\xff"

    def rec(rtype, ld1_, raw2, len2, nxt=0):
        bs = 21 + len(ld1_) + len(raw2)
        return struct.pack("<IIIIIB", bs, len2, 0, nxt, 21 + len(ld1_), rtype) + ld1_ + raw2

    pos0 = 12
    ld1_hdr = struct.pack("<iiiiiii", 0, -1, -1, 0, 0, 0, 0)
    ld2_hdr = title + b"\0"
    hsize = len(rec(2, ld1_hdr, ld2_hdr, len(ld2_hdr)))
    dsize = len(rec(0x20, ld1, disp_raw, len(ld2)))
    pos1 = pos0 + hsize
    sent_pos = pos1 + dsize
    hdr_rec = rec(2, ld1_hdr, ld2_hdr, len(ld2_hdr), pos1)
    disp = rec(0x20, ld1, disp_raw, len(ld2), sent_pos)
    sent = rec(2, struct.pack("<iiiiiii", 0, -1, -1, 1, 0, 0, 0), b"", 0, 0xFFFFFFFF)
    stream = hdr_rec + disp + sent
    comp_stream = lz_literal(stream)
    block = struct.pack("<III", 0xFFFFFFFF, pos0, pos0) + comp_stream
    topic = block.ljust(0x1000, b"\0")

    system = struct.pack("<HHHIH", 0x036C, 21, 1, 0, 4) + \
        struct.pack("<HH", 1, 5) + b"Mini\0"
    ttl = btree(b"Lz", [(0, title + b"\0")], lambda k: struct.pack("<I", k))
    ctx = btree(b"L4", [(0x11223344, struct.pack("<I", 0))], lambda k: struct.pack("<I", k))
    files = {"|SYSTEM": system, "|TOPIC": topic, "|Phrases": phrases_file,
             "|TTLBTREE": ttl, "|CONTEXT": ctx}
    # lay out: 16-byte header, then each file (9-byte file header + data), then directory
    data = bytearray(16)
    offs = {}
    for name, d in files.items():
        offs[name] = len(data)
        data += struct.pack("<IIB", len(d), len(d), 4) + d
    dir_off = len(data)
    d = btree(b"z4", sorted((n, struct.pack("<I", o)) for n, o in offs.items()),
              lambda k: k.encode() + b"\0")
    data += struct.pack("<IIB", len(d), len(d), 4) + d
    data[0:16] = struct.pack("<IIII", 0x00035F3F, dir_off, 0xFFFFFFFF, len(data))
    return bytes(data)


class Tests(unittest.TestCase):
    def test_lz77(self):
        s = b"abcdefgh" * 2
        enc = lz_with_match(b"abcdefgh", 8, 8)
        self.assertEqual(H.lz77(enc), s)
        self.assertEqual(H.lz77(lz_literal(b"xyz")), b"xyz")

    def test_compressed_numbers(self):
        self.assertEqual(H.cu(bytes([0x0A]), 0), (5, 1))
        self.assertEqual(H.cu(bytes([0x35, 0x04]), 0), (538, 2))
        self.assertEqual(H.cs(bytes([0x82]), 0), (1, 1))
        self.assertEqual(H.csl(bytes([0x18, 0x80]), 0), (12, 2))

    def test_unphrase(self):
        ph = [b"Hello", b"world"]
        self.assertEqual(H.unphrase(bytes([1, 1, 1, 2, 0x41]), ph), b"Hello world" + b"A")

    def test_mini_file(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "mini.hlp")
            with open(p, "wb") as f:
                f.write(build_hlp())
            out = os.path.join(d, "out")
            r = H.dump(p, out)
            self.assertEqual(r["topics"], 1)
            self.assertEqual(r["failed"], 0)
            names = os.listdir(out)
            self.assertIn("index.tsv", names)
            with open(os.path.join(out, [n for n in names if n.startswith("0000")][0])) as f:
                txt = f.read()
            self.assertIn("TITLE Mini Topic", txt)
            self.assertIn("Hello world!", txt)
            self.assertIn("11223344", txt)


if __name__ == "__main__":
    unittest.main()
