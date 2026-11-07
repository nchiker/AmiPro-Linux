#!/usr/bin/env python3
"""Drive the original Ami Pro reference VM's mouse through the VirtualBox API.

Usage: vmmouse.py ACTION...
  move:X,Y         move the pointer to screen pixel X,Y (0-based, as in screenshots)
  click:X,Y        left click at X,Y
  shiftclick:X,Y   left click at X,Y with Shift held
  dclick:X,Y       left double click at X,Y
  drag:X1,Y1,X2,Y2 press at X1,Y1, move in steps, release at X2,Y2
  wait:SECONDS     pause (bounded to 10 s)

Needs the VirtualBox Python bindings (vboxapi) and Guest Additions in the
guest for absolute pointing. Environment: AMIPRO_VM (default "Windows XP").
Keyboard input and screenshots are in vmkey.py; see docs/research/VM_LAB.md.
"""
import os
import sys
import time

import vboxapi

VM = os.environ.get("AMIPRO_VM", "Windows XP")
LEFT = 1
SHIFT_DOWN = ["2a"]
SHIFT_UP = ["aa"]


def main(actions):
    mgr = vboxapi.VirtualBoxManager(None, None)
    vbox = mgr.getVirtualBox()
    machine = vbox.findMachine(VM)
    session = mgr.getSessionObject()
    machine.lockMachine(session, mgr.constants.LockType_Shared)
    try:
        console = session.console
        mouse = console.mouse
        keyboard = console.keyboard

        def at(x, y, buttons=0):
            # The API is 1-based; screenshots are 0-based.
            mouse.putMouseEventAbsolute(x + 1, y + 1, 0, 0, buttons)
            time.sleep(0.05)

        def click(x, y):
            at(x, y)
            at(x, y, LEFT)
            at(x, y)

        for action in actions:
            kind, _, arg = action.partition(":")
            nums = [int(v) for v in arg.split(",")] if kind != "wait" else []
            if kind == "move":
                at(*nums)
            elif kind == "click":
                click(*nums)
            elif kind == "shiftclick":
                keyboard.putScancodes([int(c, 16) for c in SHIFT_DOWN])
                click(*nums)
                keyboard.putScancodes([int(c, 16) for c in SHIFT_UP])
            elif kind == "dclick":
                click(*nums)
                click(*nums)
            elif kind == "drag":
                x1, y1, x2, y2 = nums
                at(x1, y1)
                at(x1, y1, LEFT)
                for i in range(1, 11):
                    at(x1 + (x2 - x1) * i // 10, y1 + (y2 - y1) * i // 10, LEFT)
                at(x2, y2)
            elif kind == "wait":
                time.sleep(min(float(arg), 10.0))
            else:
                sys.exit(f"unknown action {action}")
            time.sleep(0.15)
    finally:
        session.unlockMachine()


if __name__ == "__main__":
    main(sys.argv[1:])
