#!/usr/bin/env python3
"""WinHelp 3.x (.HLP) reader: read-only fact extraction for research.

Usage: hlp_dump.py FILE.hlp [OUTDIR]     (default OUTDIR ../AmiPro-Reference/help-dump/<name>/)
Writes one text file per topic plus index.tsv, outside Git.  The output is
proprietary source text and must never be committed or embedded.  Stdlib only.

Format knowledge (independent of any other tool): file header, internal
directory (B+ tree), |SYSTEM, |TOPIC blocks (LZ77), topic links, |Phrases
phrase compression, |TTLBTREE, |CONTEXT, |CTXOMAP, |KWBTREE/|KWDATA.
"""
import os
import struct
import sys


class HlpError(Exception):
    pass


# --------------------------------------------------------------- primitives
def lz77(data, out=None):
    """Microsoft WinHelp LZ77: control byte, bit0 first; 0 literal, 1 = word
    (len-3)<<12 | (pos-1)."""
    out = bytearray() if out is None else out
    i, n = 0, len(data)
    while i < n:
        ctl = data[i]
        i += 1
        for bit in range(8):
            if i >= n:
                break
            if ctl & (1 << bit):
                if i + 2 > n:
                    i = n
                    break
                w = data[i] | (data[i + 1] << 8)
                i += 2
                pos = (w & 0x0FFF) + 1
                ln = (w >> 12) + 3
                for _ in range(ln):
                    if pos > len(out):
                        raise HlpError("bad LZ77 back reference")
                    out.append(out[-pos])
            else:
                out.append(data[i])
                i += 1
    return bytes(out)


def cu(buf, p):
    """Compressed unsigned short; returns (value, newpos)."""
    b = buf[p]
    if b & 1:
        return (b | (buf[p + 1] << 8)) >> 1, p + 2
    return b >> 1, p + 1


def cs(buf, p):
    b = buf[p]
    if b & 1:
        return ((b | (buf[p + 1] << 8)) >> 1) - 0x4000, p + 2
    return (b >> 1) - 0x40, p + 1


def cul(buf, p):
    """Compressed unsigned long."""
    b = buf[p]
    if b & 1:
        v = b | (buf[p + 1] << 8) | (buf[p + 2] << 16) | (buf[p + 3] << 24)
        return v >> 1, p + 4
    return (b | (buf[p + 1] << 8)) >> 1, p + 2


def csl(buf, p):
    b = buf[p]
    if b & 1:
        v = b | (buf[p + 1] << 8) | (buf[p + 2] << 16) | (buf[p + 3] << 24)
        return (v >> 1) - 0x4000000, p + 4
    return ((b | (buf[p + 1] << 8)) >> 1) - 0x4000, p + 2


def cstr(buf, p):
    e = buf.index(b"\0", p)
    return buf[p:e].decode("cp1252", "replace"), e + 1


# --------------------------------------------------------------- container
class Hlp:
    def __init__(self, path):
        with open(path, "rb") as f:
            self.d = f.read()
        d = self.d
        if len(d) < 16 or struct.unpack_from("<I", d, 0)[0] != 0x00035F3F:
            raise HlpError("not a WinHelp file")
        self.dir_start = struct.unpack_from("<I", d, 4)[0]
        self.files = {}
        self._read_directory()

    def _fileat(self, off):
        res, used = struct.unpack_from("<II", self.d, off)
        return self.d[off + 9: off + 9 + used]

    def _read_directory(self):
        data = self._fileat(self.dir_start)
        for key, val, _ in btree_entries(data, "z4"):
            self.files[key] = struct.unpack("<I", val)[0]

    def get(self, name):
        if name not in self.files:
            return None
        return self._fileat(self.files[name])


def btree_entries(data, fmt=None):
    """Yield (key, value_bytes, key_raw) for all leaf entries.  fmt selects the
    key shape: 'z..' = string key, 'L'/'1' etc = binary key (size via table).
    Value length is taken from the structure string (z4 = string + 4 bytes,
    L2 = long + 2...)"""
    magic, flags, psize = struct.unpack_from("<HHH", data, 0)
    if magic != 0x293B:
        raise HlpError("bad B+tree magic %04x" % magic)
    structure = data[6:22].split(b"\0")[0].decode("latin1")
    root, = struct.unpack_from("<H", data, 26)
    total_pages, nlevels = struct.unpack_from("<HH", data, 30)
    base = 38
    if fmt is None:
        fmt = structure

    def page(n):
        return base + n * psize

    cur = root
    for _ in range(nlevels - 1):  # descend through first pointers
        off = page(cur)
        cur = struct.unpack_from("<H", data, off + 4)[0]
    seen = set()
    while cur != 0xFFFF and cur not in seen and cur < max(total_pages, 1):
        seen.add(cur)
        off = page(cur)
        nent, prev, nxt = struct.unpack_from("<HHH", data, off + 2)
        p = off + 8
        for _ in range(nent):
            if fmt[0] == "z":
                e = data.index(b"\0", p)
                key = data[p:e].decode("cp1252", "replace")
                p = e + 1
            elif fmt[0] == "L":
                key = struct.unpack_from("<I", data, p)[0]
                p += 4
            elif fmt[0] == "i":
                key = struct.unpack_from("<h", data, p)[0]
                p += 2
            elif fmt[0] == "2":
                key = struct.unpack_from("<H", data, p)[0]
                p += 2
            elif fmt[0] == "4":
                key = struct.unpack_from("<I", data, p)[0]
                p += 4
            else:
                raise HlpError("unknown key type " + fmt)
            vstart = p
            rest = fmt[1:]
            if rest == "":
                p = None  # caller must supply; unsupported
                break
            if rest.isdigit():
                p += int(rest)
            elif rest[0] == "z":
                p = data.index(b"\0", p) + 1
            else:
                raise HlpError("unknown value shape " + fmt)
            yield key, data[vstart:p], None
        cur = nxt
    return


# --------------------------------------------------------------- tables
def read_system(h):
    s = h.get("|SYSTEM")
    info = {"title": None, "minor": None, "flags": 0, "records": []}
    if not s:
        return info
    magic, minor, major, gen, flags = struct.unpack_from("<HHHIH", s, 0)
    info.update(minor=minor, major=major, flags=flags)
    if minor <= 16:
        try:
            info["title"] = cstr(s, 12)[0]
        except ValueError:
            pass
        return info
    p = 12
    while p + 4 <= len(s):
        t, sz = struct.unpack_from("<HH", s, p)
        p += 4
        body = s[p:p + sz]
        p += sz
        info["records"].append((t, body))
        if t == 1 and info["title"] is None:
            info["title"] = body.split(b"\0")[0].decode("cp1252", "replace")
    return info


def read_phrases(h):
    """Return list of phrase byte strings or None."""
    ph = h.get("|Phrases")
    if not ph:
        return None
    n, hundred, decomp = struct.unpack_from("<HHI", ph, 0)
    table_len = 2 * (n + 1)
    table = ph[8:8 + table_len]
    rest = ph[8 + table_len:]
    if table_len + len(rest) != decomp:
        try:
            rest = lz77(rest)
        except HlpError:
            pass
    body = table + rest
    offs = struct.unpack_from("<%dH" % (n + 1), table, 0)
    phrases = []
    for i in range(n):
        phrases.append(body[offs[i]:offs[i + 1]])
    return phrases


def unphrase(data, phrases):
    if not phrases:
        return data
    out = bytearray()
    i, n = 0, len(data)
    while i < n:
        b = data[i]
        if 1 <= b <= 15 and i + 1 < n:
            idx = (b - 1) * 128 + (data[i + 1] >> 1)
            sp = data[i + 1] & 1
            i += 2
            if idx < len(phrases):
                out += phrases[idx]
            if sp:
                out.append(0x20)
        else:
            out.append(b)
            i += 1
    return bytes(out)


# --------------------------------------------------------------- topic
BLOCK_STRIDE = 0x1000


def read_topic_stream(h, sysinfo):
    t = h.get("|TOPIC")
    if t is None:
        raise HlpError("no |TOPIC")
    minor = sysinfo["minor"] or 21
    stride = 0x800 if minor <= 16 else 0x1000
    compressed = minor > 16 and sysinfo["flags"] in (4, 8)
    blocks = []
    for off in range(0, len(t), stride):
        blk = t[off:off + stride]
        if len(blk) < 12:
            break
        hdr = struct.unpack_from("<III", blk, 0)
        body = blk[12:]
        if compressed:
            try:
                body = lz77(body)
            except HlpError:
                pass
        blocks.append((hdr, body))
    return blocks, compressed


class Stream:
    """Concatenated decompressed blocks addressed by TOPICPOS."""

    def __init__(self, blocks, compressed):
        self.blocks = blocks
        self.size = 0x4000
        self.data = bytearray()
        for _, body in blocks:
            # every block but the last occupies exactly size-12 bytes
            self.data += body
        self.chunk = (self.size - 12)
        self.sizes = [len(b) for _, b in blocks]

    def idx(self, pos):
        blk, off = pos >> 14, pos & 0x3FFF
        start = sum(self.sizes[:blk])
        return start + off - 12

    def at(self, pos, n):
        i = self.idx(pos)
        return bytes(self.data[i:i + n])


LINK = struct.Struct("<IIIII B")  # 21 bytes: blocksize, datalen2, prev, next, datalen1, type


def parse_links(stream, first_pos):
    """Walk the chain from first_pos; yield dicts with pos and record parts."""
    pos = first_pos
    seen = set()
    while pos not in seen:
        seen.add(pos)
        try:
            i = stream.idx(pos)
            if i < 0 or i + 21 > len(stream.data):
                return
            bs, dl2, prev, nxt, dl1, rt = LINK.unpack_from(stream.data, i)
            if bs < 21 or dl1 < 21 or dl1 > bs:
                return
            ld1 = bytes(stream.data[i + 21: i + dl1])
            raw2 = bytes(stream.data[i + dl1: i + bs])
        except (IndexError, struct.error):
            return
        yield dict(pos=pos, size=bs, len2=dl2, prev=prev, next=nxt, type=rt,
                   ld1=ld1, raw2=raw2)
        if nxt in (0, 0xFFFFFFFF) or nxt == pos:
            return
        pos = nxt


def decode_ld2(raw2, len2, phrases):
    if len(raw2) == len2:
        return raw2
    out = unphrase(raw2, phrases)
    return out


# paragraph decoding ---------------------------------------------------------
PARA_FIELDS = ((0x0002, "cs"), (0x0004, "cs"), (0x0008, "cs"), (0x0010, "cs"),
               (0x0020, "cs"), (0x0040, "cs"))


def parse_para_info(ld1):
    """Skip TopicSize, TopicLength, the fixed pad and paragraph formatting;
    return the offset of the first command in LinkData1."""
    p = 0
    _, p = csl(ld1, p)
    _, p = cu(ld1, p)
    p += 4
    flags = ld1[p] | (ld1[p + 1] << 8)
    p += 2
    return skip_flags(ld1, p, flags)


def skip_flags(ld1, p, flags):
    if flags & 0x0001:
        _, p = csl(ld1, p)
    for bit, _k in PARA_FIELDS:
        if flags & bit:
            _, p = cs(ld1, p)
    if flags & 0x0100:
        p += 3
    if flags & 0x0200:
        n, p = cs(ld1, p)
        for _ in range(n):
            x, p = cu(ld1, p)
            if x & 0x4000:
                _, p = cu(ld1, p)
    return p


class ParaError(Exception):
    pass


def parse_commands(ld1, p, strings, ctx, upto=None):
    """Interpret formatting commands from ld1[p:]; each command (and the final
    0xFF) first emits the next NUL-terminated string of LinkData2.
    Returns (text, links)."""
    out = []
    links = []
    hot = None  # (kind, target, text pieces)
    n = len(ld1)

    def take():
        try:
            return next(strings)
        except StopIteration:
            return ""

    while p < n:
        c = ld1[p]
        p += 1
        t = take()
        if t:
            out.append(t)
            if hot is not None:
                hot[2].append(t)
        if c == 0xFF:
            break
        if c == 0x80:
            p += 2
        elif c in (0x81, 0x82):
            out.append("\n")
        elif c == 0x83:
            out.append("\t")
        elif c in (0x20, 0x21):
            _, p = csl(ld1, p)
        elif c in (0x86, 0x87, 0x88):
            typ = ld1[p]
            p += 1
            sz, p = csl(ld1, p)
            p += max(sz, 0)
            while p < n and ld1[p] == 0:  # pad byte after the picture data
                p += 1
            out.append("[picture]")
        elif c == 0x89:
            if hot is not None:
                links.append((hot[0], hot[1], "".join(hot[2])))
                hot = None
        elif c == 0x8B:
            out.append("\u00a0")
        elif c == 0x8C:
            out.append("-")
        elif c in (0xC8, 0xCC):
            ln = ld1[p] | (ld1[p + 1] << 8)
            p += 2
            m = ld1[p:p + ln].split(b"\0")[0].decode("cp1252", "replace")
            p += ln
            hot = ("macro", m, [])
        elif c in (0xE0, 0xE1, 0xE2, 0xE3, 0xE6, 0xE7):
            h = struct.unpack_from("<I", ld1, p)[0]
            p += 4
            hot = ("jump" if c & 1 else "popup", h, [])
        elif c in (0xEA, 0xEB, 0xEE, 0xEF):
            ln = ld1[p] | (ld1[p + 1] << 8)
            body = ld1[p + 2:p + 2 + ln]
            p += 2 + ln
            typ = body[0] if body else 0
            h = struct.unpack_from("<I", body, 1)[0] if len(body) >= 5 else 0
            extra = body[5:].split(b"\0")[0].decode("cp1252", "replace")
            if typ == 1 and len(body) >= 6:
                extra = "window%d" % body[5]
            hot = ("ext-" + ("jump" if c & 1 else "popup"), (h, extra), [])
        else:
            raise ParaError("unknown command %02x" % c)
    return "".join(out), links, p


def parse_table(ld1, strings):
    """Table record: header, then cells (column, 0x01, para info, commands,
    ending 0xFF) until a column of -1.  Returns (text, links)."""
    p = 0
    _, p = csl(ld1, p)
    _, p = cu(ld1, p)
    ncols = ld1[p]
    ttype = ld1[p + 1]
    p += 2
    if ttype in (0, 2):
        p += 2
    p += 4 * ncols
    rows = []
    links = []
    lastcol = 99
    while p + 2 <= len(ld1):
        col = struct.unpack_from("<h", ld1, p)[0]
        p += 2
        if col == -1:
            break
        p += 1
        _, p = csl(ld1, p)
        _, p = cu(ld1, p)
        p += 3
        flags = ld1[p] | (ld1[p + 1] << 8)
        p += 2
        p = skip_flags(ld1, p, flags)
        text, lk, p = parse_commands(ld1, p, strings, None)
        links += lk
        if col <= lastcol or not rows:
            rows.append([])
        lastcol = col
        text = text.strip()
        if text:
            rows[-1].append(text.replace("\n", " ").replace("\t", " "))
    return "\n".join(" | ".join(r) for r in rows if r) + "\n", links


def decode_record(rec, phrases, ctx=None):
    """Return (text, links) for a 0x20/0x01 display record."""
    ld2 = decode_ld2(rec["raw2"], rec["len2"], phrases)
    strings = [x.decode("cp1252", "replace") for x in ld2.split(b"\0")]
    it = iter(strings)
    if rec["type"] == 0x23:
        return parse_table(rec["ld1"], it)
    p = parse_para_info(rec["ld1"])
    text, links, _ = parse_commands(rec["ld1"], p, it, ctx)
    return text, links


# --------------------------------------------------------------- top level
def build_topics(h, warn):
    si = read_system(h)
    if si["minor"] is None or si["minor"] <= 16:
        warn.append("WinHelp 3.0 (minor %s) topic records are not supported" % si["minor"])
    blocks, compressed = read_topic_stream(h, si)
    stream = Stream(blocks, compressed)
    phrases = read_phrases(h)
    first = blocks[0][0][1] if blocks else 12
    first = first if first not in (0, 0xFFFFFFFF) else 12
    topics = []
    cur = None
    for rec in parse_links(stream, first):
        rt = rec["type"]
        if rt == 2:
            ld2 = decode_ld2(rec["raw2"], rec["len2"], phrases)
            title = ld2.split(b"\0")[0].decode("cp1252", "replace")
            hd = rec["ld1"]
            num = struct.unpack_from("<i", hd, 12)[0] if len(hd) >= 16 else len(topics)
            browse = struct.unpack_from("<ii", hd, 4) if len(hd) >= 12 else (-1, -1)
            cur = dict(pos=rec["pos"], title=title, num=num, browse=browse,
                       records=[], text="", links=[], errors=[], offset=None,
                       sentinel=(rec["next"] == 0xFFFFFFFF and rec["len2"] == 0))
            topics.append(cur)
        elif cur is not None and rt in (0x20, 0x23, 0x01):
            cur["records"].append(rec)
    for t in topics:
        parts = []
        for rec in t["records"]:
            try:
                txt, lk = decode_record(rec, phrases)
                parts.append(txt)
                t["links"] += lk
            except Exception as e:  # keep going; report per topic
                t["errors"].append("record %x: %s" % (rec["pos"], e))
                try:
                    ld2 = decode_ld2(rec["raw2"], rec["len2"], phrases)
                    parts.append(" ".join(x.decode("cp1252", "replace")
                                          for x in ld2.split(b"\0") if x))
                except Exception:
                    pass
        t["text"] = "".join(parts)
    # titles and offsets
    real = [t for t in topics if not t["sentinel"]]
    ttl = h.get("|TTLBTREE")
    keys = []
    titles = {}
    if ttl:
        for k, v, _ in btree_entries(ttl, "Lz"):
            keys.append(k)
            titles[k] = v.rstrip(b"\0").decode("cp1252", "replace")
        keys.sort()
    if len(keys) >= len(real):
        for t, k in zip(real, keys):
            t["offset"] = k
            if k in titles and titles[k]:
                t["title"] = titles[k]
    else:
        warn.append("TTLBTREE has %d keys for %d topics" % (len(keys), len(real)))
    # consistency of browse chain with offsets
    bad = 0
    for i, t in enumerate(real[:-1]):
        nxt = t["browse"][1]
        if nxt != -1 and t["offset"] is not None and real[i + 1]["offset"] is not None \
                and nxt != real[i + 1]["offset"]:
            bad += 1
    if bad:
        warn.append("%d topics whose browse-next differs from the next topic offset" % bad)
    return si, real, phrases


def read_context(h):
    c = h.get("|CONTEXT")
    out = {}
    if c:
        for k, v, _ in btree_entries(c, "L4"):
            out[k] = struct.unpack("<I", v)[0]
    return out


def read_keywords(h):
    kw = h.get("|KWBTREE")
    kd = h.get("|KWDATA")
    res = []  # (keyword, [offsets])
    if not kw or not kd:
        return res
    for k, v, _ in btree_entries(kw, "z6"):
        cnt, off = struct.unpack("<HI", v)
        offs = [struct.unpack_from("<I", kd, off + 4 * i)[0]
                for i in range(cnt) if off + 4 * i + 4 <= len(kd)]
        res.append((k, offs))
    return res


def read_ctxomap(h):
    d = h.get("|CTXOMAP")
    if not d or len(d) < 2:
        return []
    n = struct.unpack_from("<H", d, 0)[0]
    return [struct.unpack_from("<II", d, 2 + 8 * i) for i in range(n)
            if 2 + 8 * i + 8 <= len(d)]


def slug(s):
    out = "".join(c if c.isalnum() else "_" for c in s.lower())
    out = "_".join(x for x in out.split("_") if x)
    return out[:48] or "untitled"


def dump(path, outdir):
    h = Hlp(path)
    warn = []
    si, topics, phrases = build_topics(h, warn)
    ctx = read_context(h)
    by_off = {t["offset"]: t for t in topics if t["offset"] is not None}
    hashes = {}
    for hv, off in ctx.items():
        hashes.setdefault(off, []).append(hv)
    kws = read_keywords(h)
    topic_kws = {}
    for k, offs in kws:
        for o in offs:
            topic_kws.setdefault(o, []).append(k)
    os.makedirs(outdir, exist_ok=True)
    rows = []
    failed = 0
    for i, t in enumerate(topics):
        fname = "%04d_%s.txt" % (i, slug(t["title"]))
        lines = ["TOPIC %d" % i, "TITLE %s" % t["title"],
                 "OFFSET %s" % ("-" if t["offset"] is None else "0x%x" % t["offset"]),
                 "HASH %s" % " ".join("%08x" % x for x in hashes.get(t["offset"], [])),
                 "KEYWORDS %s" % "; ".join(topic_kws.get(t["offset"], [])), ""]
        lines.append(t["text"].rstrip("\n"))
        lines.append("")
        lines.append("LINKS")
        for lk in t["links"]:
            kind, target, label = lk
            if kind == "macro":
                lines.append("macro\t%s\t%s" % (target, label))
            elif kind.startswith("ext-"):
                lines.append("%s\t%08x %s\t%s" % (kind, target[0], target[1], label))
            else:
                off = ctx.get(target)
                tt = by_off.get(off)
                if tt is not None:
                    lines.append("%s\t#%d %s\t%s" % (kind, topics.index(tt), tt["title"], label))
                else:
                    lines.append("%s\t%08x (unresolved)\t%s" % (kind, target, label))
        if t["errors"]:
            lines.append("")
            lines.append("ERRORS " + "; ".join(t["errors"]))
            failed += 1
        with open(os.path.join(outdir, fname), "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")
        rows.append((i, "-" if t["offset"] is None else "0x%x" % t["offset"],
                     " ".join("%08x" % x for x in hashes.get(t["offset"], [])),
                     t["title"], len(t["text"]), len(t["links"]),
                     "FAIL" if t["errors"] else "ok", fname))
    with open(os.path.join(outdir, "index.tsv"), "w", encoding="utf-8") as f:
        f.write("num\toffset\thash\ttitle\tchars\tlinks\tstatus\tfile\n")
        for r in rows:
            f.write("\t".join(str(x).replace("\t", " ") for x in r) + "\n")
    with open(os.path.join(outdir, "_system.txt"), "w", encoding="utf-8") as f:
        f.write("title\t%s\nversion\t3.%s\n" % (si["title"], si["minor"]))
        for t_, body in si["records"]:
            if t_ in (1, 2, 4, 6, 8, 5):
                f.write("rec%d\t%s\n" % (t_, body.split(b"\0")[0].decode("cp1252", "replace")))
        f.write("ctxomap_entries\t%d\n" % len(read_ctxomap(h)))
        f.write("keywords\t%d\n" % len(kws))
        f.write("internal_files\t%s\n" % " ".join(sorted(h.files)))
        for w in warn:
            f.write("warning\t%s\n" % w)
    return dict(topics=len(topics), ok=len(topics) - failed, failed=failed,
                keywords=len(kws), contexts=len(ctx), warnings=warn,
                sample=[t["title"] for t in topics[:5]])


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    path = argv[1]
    name = os.path.splitext(os.path.basename(path))[0].lower()
    here = os.path.dirname(os.path.abspath(__file__))
    outdir = argv[2] if len(argv) > 2 else os.path.normpath(
        os.path.join(here, "..", "..", "AmiPro-Reference", "help-dump", name))
    r = dump(path, outdir)
    print("%s: topics=%d ok=%d failed=%d keywords=%d contexts=%d warnings=%s -> %s" % (
        name, r["topics"], r["ok"], r["failed"], r["keywords"], r["contexts"],
        r["warnings"], outdir))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
