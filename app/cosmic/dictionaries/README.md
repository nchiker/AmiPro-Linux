# Bundled dictionary

`en_US.aff` and `en_US.dic` are the free US English Hunspell dictionary
built from SCOWL (Spell Checker Oriented Word Lists) by Kevin Atkinson and
contributors, as packaged by Debian/Ubuntu (`hunspell-en-us`
2020.12.07). Its permissive licence and the notices of its sources are in
`en_US-COPYRIGHT.txt`. The app bundles it so that English spelling works
when no system dictionary is installed (decision SP1,
`docs/research/DECISIONS.md`); system dictionaries in
`/usr/share/hunspell` and `~/.local/share/hunspell` are preferred.
