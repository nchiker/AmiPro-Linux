---
id: password-protection
title: Password protection
area: files
keywords: password, protect, encrypt, lock, secure
see_also: saving-files, crash-recovery, opening-files
context: command:Password
---
**File › Password…** protects an Ami Pro document so that opening it asks for a password. The scheme is Ami Pro's own, so Ami Pro can open the file. It only scrambles the text, which is not real security, so do not rely on it for secrets.

## Setting and removing

Choose File › Password…, type the password in **Enter**, repeat it in **Verify**, and confirm. A password can be up to 14 characters. From then on every save of that document as an Ami Pro file is written protected. To remove the protection, open the same command and choose **Remove Password**, then save.

## Things to know

- Opening a protected file asks for the password; a wrong one does not open it.
- Protection applies to Ami Pro (`.SAM`) saves. Exports to other formats are not protected.
- A protected document is never copied by [crash recovery](crash-recovery), because the copy would not be protected.
- Forgotten passwords cannot be recovered by the program.
