#!/usr/bin/env python3
"""Read-only ISO-9660 inventory helper for private Ami Pro reference media."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import BinaryIO, Iterator


SECTOR_SIZE = 2048


@dataclass(frozen=True)
class Entry:
    path: PurePosixPath
    extent: int
    size: int
    is_dir: bool


def read_record(data: bytes, offset: int, parent: PurePosixPath) -> tuple[Entry | None, int]:
    length = data[offset]
    if length == 0:
        return None, ((offset // SECTOR_SIZE) + 1) * SECTOR_SIZE
    record = data[offset : offset + length]
    if len(record) != length or length < 34:
        raise ValueError(f"truncated ISO directory record at byte {offset}")
    extent = struct.unpack_from("<I", record, 2)[0]
    size = struct.unpack_from("<I", record, 10)[0]
    flags = record[25]
    name_length = record[32]
    raw_name = record[33 : 33 + name_length]
    if raw_name in (b"\x00", b"\x01"):
        return None, offset + length
    name = raw_name.decode("ascii", errors="replace").split(";", 1)[0]
    return Entry(parent / name, extent, size, bool(flags & 0x02)), offset + length


def children(handle: BinaryIO, directory: Entry) -> Iterator[Entry]:
    handle.seek(directory.extent * SECTOR_SIZE)
    data = handle.read(directory.size)
    offset = 0
    while offset < len(data):
        entry, next_offset = read_record(data, offset, directory.path)
        if next_offset <= offset:
            raise ValueError("invalid ISO directory record length")
        offset = next_offset
        if entry is not None:
            yield entry


def inventory(handle: BinaryIO) -> list[Entry]:
    handle.seek(16 * SECTOR_SIZE)
    pvd = handle.read(SECTOR_SIZE)
    if len(pvd) != SECTOR_SIZE or pvd[0] != 1 or pvd[1:6] != b"CD001":
        raise ValueError("not an ISO-9660 primary volume descriptor")
    root_data = pvd[156:]
    root_length = root_data[0]
    root = Entry(PurePosixPath("/"), struct.unpack_from("<I", root_data, 2)[0],
                 struct.unpack_from("<I", root_data, 10)[0], True)
    result: list[Entry] = []
    pending = [root]
    seen_extents: set[int] = set()
    while pending:
        directory = pending.pop()
        if directory.extent in seen_extents:
            continue
        seen_extents.add(directory.extent)
        for entry in children(handle, directory):
            result.append(entry)
            if entry.is_dir:
                pending.append(entry)
    return sorted(result, key=lambda item: str(item.path).casefold())


def digest_extent(handle: BinaryIO, entry: Entry) -> str:
    handle.seek(entry.extent * SECTOR_SIZE)
    remaining = entry.size
    digest = hashlib.sha256()
    while remaining:
        block = handle.read(min(1024 * 1024, remaining))
        if not block:
            raise ValueError(f"truncated extent for {entry.path}")
        digest.update(block)
        remaining -= len(block)
    return digest.hexdigest()


def read_extent(handle: BinaryIO, entry: Entry) -> bytes:
    handle.seek(entry.extent * SECTOR_SIZE)
    data = handle.read(entry.size)
    if len(data) != entry.size:
        raise ValueError(f"truncated extent for {entry.path}")
    return data


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("iso", type=Path)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--hashes", action="store_true", help="hash every regular file")
    parser.add_argument("--extract", metavar="ISO_PATH", help="copy one file to --output for private analysis")
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--extract-extension",
        action="append",
        metavar="EXT",
        help="extract all regular files with EXT into a new --output-dir (repeatable)",
    )
    parser.add_argument("--output-dir", type=Path)
    args = parser.parse_args()

    with args.iso.open("rb") as handle:
        entries = inventory(handle)
        if args.extract:
            if args.output is None:
                parser.error("--extract requires --output")
            if args.output.exists():
                parser.error(f"refusing to overwrite existing output: {args.output}")
            matches = [entry for entry in entries if str(entry.path).casefold() == args.extract.casefold()]
            if len(matches) != 1 or matches[0].is_dir:
                parser.error(f"regular file not found: {args.extract}")
            entry = matches[0]
            with args.output.open("xb") as output:
                output.write(read_extent(handle, entry))
            return
        if args.extract_extension:
            if args.output_dir is None:
                parser.error("--extract-extension requires --output-dir")
            if args.output_dir.exists():
                parser.error(f"refusing to use existing output directory: {args.output_dir}")
            extensions = {
                extension.casefold() if extension.startswith(".") else f".{extension.casefold()}"
                for extension in args.extract_extension
            }
            matches = [
                entry
                for entry in entries
                if not entry.is_dir and entry.path.suffix.casefold() in extensions
            ]
            args.output_dir.mkdir(parents=True)
            for entry in matches:
                relative = Path(*entry.path.parts[1:])
                output_path = args.output_dir / relative
                output_path.parent.mkdir(parents=True, exist_ok=True)
                with output_path.open("xb") as output:
                    output.write(read_extent(handle, entry))
            print(f"extracted {len(matches)} files to {args.output_dir}")
            return
        rows = []
        for entry in entries:
            row = {
                "path": str(entry.path),
                "type": "directory" if entry.is_dir else "file",
                "size": entry.size,
            }
            if args.hashes and not entry.is_dir:
                row["sha256"] = digest_extent(handle, entry)
            rows.append(row)
    if args.json:
        print(json.dumps(rows, indent=2))
    else:
        for row in rows:
            digest = f"\t{row['sha256']}" if "sha256" in row else ""
            print(f"{row['type'][0]}\t{row['size']}\t{row['path']}{digest}")


if __name__ == "__main__":
    main()
