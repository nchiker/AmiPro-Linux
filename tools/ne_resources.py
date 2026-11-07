#!/usr/bin/env python3
"""Lists the dialog templates and string tables in a 16-bit NE executable
(read-only; used for Ami Pro's dialogs in AMIPROUI.DLL, docs/research/PAGE_LAYOUT_NOTES.md).

Usage: tools/ne_resources.py FILE [dialogs|strings|menus|types] [FILTER]
"""
import struct
import sys

CLASSES = {0x80: "BUTTON", 0x81: "EDIT", 0x82: "STATIC", 0x83: "LISTBOX",
           0x84: "SCROLLBAR", 0x85: "COMBOBOX"}


def resources(data):
    ne = struct.unpack_from("<I", data, 0x3C)[0]
    assert data[ne:ne + 2] == b"NE"
    rtab = ne + struct.unpack_from("<H", data, ne + 0x24)[0]
    shift = struct.unpack_from("<H", data, rtab)[0]
    pos = rtab + 2
    out = []
    while True:
        tid = struct.unpack_from("<H", data, pos)[0]
        if tid == 0:
            break
        count = struct.unpack_from("<H", data, pos + 2)[0]
        pos += 8
        for _ in range(count):
            off, length, flags, rid = struct.unpack_from("<HHHH", data, pos)
            pos += 12
            if rid & 0x8000:
                name = rid & 0x7FFF
            else:
                n = data[rtab + rid]
                name = data[rtab + rid + 1:rtab + rid + 1 + n].decode("latin-1")
            ttype = tid & 0x7FFF if tid & 0x8000 else tid
            out.append((ttype, name, off << shift, length << shift))
    return out


def cstr(data, pos):
    end = data.index(b"\0", pos)
    return data[pos:end].decode("cp1252"), end + 1


def name_or_ord(data, pos):
    if data[pos] == 0xFF:
        return "#%d" % struct.unpack_from("<H", data, pos + 1)[0], pos + 3
    return cstr(data, pos)


def dialog(data, off):
    style, count, x, y, cx, cy = struct.unpack_from("<IBHHHH", data, off)
    pos = off + 13
    menu, pos = name_or_ord(data, pos)
    cls, pos = cstr(data, pos)
    caption, pos = cstr(data, pos)
    if style & 0x40:
        size = struct.unpack_from("<H", data, pos)[0]
        face, pos = cstr(data, pos + 2)
    lines = ['  caption "%s" (%d,%d %dx%d) style %08x' % (caption, x, y, cx, cy, style)]
    for _ in range(count):
        ix, iy, icx, icy, iid, istyle = struct.unpack_from("<HHHHHI", data, pos)
        pos += 14
        if data[pos] & 0x80:
            icls = CLASSES.get(data[pos], hex(data[pos]))
            pos += 1
        else:
            icls, pos = cstr(data, pos)
        if data[pos] == 0xFF:
            text = "#%d" % struct.unpack_from("<H", data, pos + 1)[0]
            pos += 3
        else:
            text, pos = cstr(data, pos)
        extra = data[pos]
        pos += 1 + extra
        kind = icls
        if icls == "BUTTON":
            bs = istyle & 0xF
            kind = {0: "PUSH", 1: "DEFPUSH", 2: "CHECK", 3: "AUTOCHECK", 4: "RADIO",
                    7: "GROUP", 9: "AUTORADIO", 11: "OWNER"}.get(bs, "BTN%d" % bs)
        lines.append("  %5d %-10s %-28s (%d,%d %dx%d) %08x" % (
            iid, kind, '"%s"' % text, ix, iy, icx, icy, istyle))
    return lines


def menu(data, off):
    """A Win16 menu template: items as (depth, flags, id, text)."""
    pos = off + 4 + struct.unpack_from("<H", data, off + 2)[0]
    out, depth, ends = [], 0, []
    while True:
        flags = struct.unpack_from("<H", data, pos)[0]
        pos += 2
        iid = None
        if not flags & 0x10:
            iid = struct.unpack_from("<H", data, pos)[0]
            pos += 2
        text, pos = cstr(data, pos)
        out.append((depth, flags, iid, text))
        if flags & 0x10:
            ends.append(flags & 0x80)
            depth += 1
            continue
        if flags & 0x80:
            while True:
                if not ends:
                    return out
                depth -= 1
                if not ends.pop():
                    break


def main():
    path = sys.argv[1]
    what = sys.argv[2] if len(sys.argv) > 2 else "types"
    filt = sys.argv[3].lower() if len(sys.argv) > 3 else None
    data = open(path, "rb").read()
    res = resources(data)
    if what == "types":
        seen = {}
        for t, n, o, l in res:
            seen[t] = seen.get(t, 0) + 1
        for t, c in sorted(seen.items(), key=lambda kv: str(kv[0])):
            print(t, c)
    elif what == "dialogs":
        for t, n, o, l in res:
            if t != 5:
                continue
            try:
                lines = dialog(data, o)
            except Exception as e:  # noqa: BLE001
                lines = ["  (unreadable: %s)" % e]
            text = "\n".join(lines)
            if filt and filt not in text.lower() and filt != str(n).lower():
                continue
            print("DIALOG %s" % n)
            print(text)
    elif what == "menus":
        for t, n, o, l in res:
            if t != 4:
                continue
            print("MENU %s" % n)
            for depth, flags, iid, text in menu(data, o):
                if filt and filt not in text.lower():
                    continue
                ident = "" if iid is None else "0x%x" % iid
                print("  %s%-8s %s" % ("  " * depth, ident, text.replace("\t", " <tab> ")))
    elif what == "strings":
        for t, n, o, l in res:
            if t != 6:
                continue
            pos = o
            for i in range(16):
                ln = data[pos]
                s = data[pos + 1:pos + 1 + ln].decode("cp1252")
                pos += 1 + ln
                if ln and (not filt or filt in s.lower()):
                    print("%5d %s" % ((n - 1) * 16 + i, s))


main()
