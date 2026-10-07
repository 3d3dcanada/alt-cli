# Alt 0.6 observed terminal screens

These PNGs render buffers captured from an actual PTY running the packaged Alt
executable. The accompanying text is the same observed buffer; `BUILD.json`
identifies the executable's compiled inputs. No model output or success result
was invented for the images. The practice repair was entered through the file
editor, then its independent four-case assertion actually passed.

The capture covers Home, the practice guide and verified Task results at 120×40,
80×24 and 60×18. Terminal automation does not establish uncoached human usability.

To reproduce using optional Pillow/pyte development dependencies:

```bash
ALT_TEST_BINARY=/absolute/path/to/alt ALT_SCREENSHOT_DIR=/tmp/alt-screens \
  python3 scripts/smoke_practice_tui.py
```

The script drains the terminal frame before capture. Rendering uses DejaVu Sans
Mono; a user's terminal may render colors, glyphs and spacing differently.
