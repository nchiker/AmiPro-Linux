#!/usr/bin/env python3
"""Drive the original Ami Pro reference VM's keyboard and take screenshots.

Usage: vmkey.py ACTION...
  text:STRING      type a string (VBoxManage keyboardputstring, US layout)
  key:NAME         press and release a key (enter, esc, tab, f1..f12, up, ...)
  combo:A+B+C      hold modifiers and press the last key (e.g. alt+f, ctrl+s, win+r)
  altcode:NNNN     type a Windows Alt+numpad code (Num Lock must be on)
  wait:SECONDS     pause between actions (host-side, bounded to 10 s)
  shot:NAME        save a screenshot as SCREENS/NAME.png

Environment: AMIPRO_VM (default "Windows XP") and AMIPRO_VM_LAB (default
../AmiPro-Reference/vm-lab beside the repository). Screenshots stay there,
outside Git. See docs/research/VM_LAB.md.
"""
import os
import subprocess
import sys
import time

VM = os.environ.get("AMIPRO_VM", "Windows XP")
LAB = os.environ.get(
    "AMIPRO_VM_LAB",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "AmiPro-Reference", "vm-lab"),
)
SCREENS = os.path.join(LAB, "screens")

# Set-1 make codes; extended keys carry an e0 prefix.
KEYS = {
    "esc": "01", "1": "02", "2": "03", "3": "04", "4": "05", "5": "06", "6": "07",
    "7": "08", "8": "09", "9": "0a", "0": "0b", "minus": "0c", "equals": "0d",
    "backspace": "0e", "tab": "0f", "q": "10", "w": "11", "e": "12", "r": "13",
    "t": "14", "y": "15", "u": "16", "i": "17", "o": "18", "p": "19", "[": "1a",
    "]": "1b", "enter": "1c", "ctrl": "1d", "a": "1e", "s": "1f", "d": "20",
    "f": "21", "g": "22", "h": "23", "j": "24", "k": "25", "l": "26", ";": "27",
    "'": "28", "`": "29", "shift": "2a", "\\": "2b", "z": "2c", "x": "2d",
    "c": "2e", "v": "2f", "b": "30", "n": "31", "m": "32", ",": "33", ".": "34",
    "/": "35", "alt": "38", "space": "39", "caps": "3a", "f1": "3b", "f2": "3c",
    "f3": "3d", "f4": "3e", "f5": "3f", "f6": "40", "f7": "41", "f8": "42",
    "f9": "43", "f10": "44", "f11": "57", "f12": "58",
    "home": "e0 47", "up": "e0 48", "pgup": "e0 49", "left": "e0 4b",
    "right": "e0 4d", "end": "e0 4f", "down": "e0 50", "pgdn": "e0 51",
    "insert": "e0 52", "delete": "e0 53", "win": "e0 5b", "apps": "e0 5d",
}


def make(name):
    return KEYS[name].split()


def brk(name):
    codes = KEYS[name].split()
    return codes[:-1] + [f"{int(codes[-1], 16) | 0x80:02x}"]


def put(codes):
    subprocess.run(["VBoxManage", "controlvm", VM, "keyboardputscancode", *codes], check=True)


def main(actions):
    for action in actions:
        kind, _, arg = action.partition(":")
        if kind == "text":
            subprocess.run(["VBoxManage", "controlvm", VM, "keyboardputstring", arg], check=True)
        elif kind == "key":
            put(make(arg) + brk(arg))
        elif kind == "combo":
            names = arg.split("+")
            codes = []
            for n in names:
                codes += make(n)
            for n in reversed(names):
                codes += brk(n)
            put(codes)
        elif kind == "altcode":
            pad = {"0": "52", "1": "4f", "2": "50", "3": "51", "4": "4b", "5": "4c",
                   "6": "4d", "7": "47", "8": "48", "9": "49"}
            codes = ["38"]
            for d in arg:
                codes += [pad[d], f"{int(pad[d], 16) | 0x80:02x}"]
            put(codes + ["b8"])
        elif kind == "wait":
            time.sleep(min(float(arg), 10.0))
        elif kind == "shot":
            subprocess.run(["VBoxManage", "controlvm", VM, "screenshotpng", f"{SCREENS}/{arg}.png"], check=True)
        else:
            sys.exit(f"unknown action {action}")
        time.sleep(0.15)


if __name__ == "__main__":
    main(sys.argv[1:])
