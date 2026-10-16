"""Build the README demo GIF from stage frames.

  PREVIEW_JSON=1 PREVIEW_DIR=<dir> PREVIEW="94x18@0,94x18@3,..." \
      cargo test -q preview_png -- --ignored
  python tools/make_gif.py <dir> docs/demo.gif

Halfblock cells become two flat pixels; text cells are drawn with a monospace font."""
import glob
import json
import os
import sys

from PIL import Image, ImageDraw, ImageFont

CW, CH = 8, 16  # pixels per terminal cell
FONTS = [r"C:\Windows\Fonts\CascadiaMono.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]
SYMBOL_FONTS = [r"C:\Windows\Fonts\seguisym.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"]


def load(paths, size):
    for p in paths:
        if os.path.exists(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


EMOJI_FONTS = [r"C:\Windows\Fonts\seguiemj.ttf", "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"]
mono = load(FONTS, 13)
symbols = load(SYMBOL_FONTS, 12)
emoji = load(EMOJI_FONTS, 12)
# Box drawing, blocks and punctuation come from the mono font; other symbols don't.
MONO_EXTRA = set("─━│▓░█▀·…—▶")


def font_for(ch):
    if ord(ch) >= 0x1F000:
        return emoji
    if ord(ch) < 0x2000 or ch in MONO_EXTRA:
        return mono
    return symbols


def frame(doc):
    w, h = doc["w"], doc["h"]
    img = Image.new("RGB", (w * CW, h * CH))
    d = ImageDraw.Draw(img)
    for i, (sym, fg, bg) in enumerate(doc["cells"]):
        x, y = (i % w) * CW, (i // w) * CH
        if sym == "▀":
            d.rectangle([x, y, x + CW - 1, y + CH // 2 - 1], fill=fg)
            d.rectangle([x, y + CH // 2, x + CW - 1, y + CH - 1], fill=bg)
            continue
        if sym == "" or bg == "#000000":
            continue  # second half of a wide glyph (reset style): already painted
        cells = doc["cells"]
        nxt = cells[i + 1] if i + 1 < len(cells) and (i + 1) % w else None
        wide = nxt is not None and (nxt[0] == "" or nxt[2] == "#000000")
        d.rectangle([x, y, x + CW * (2 if wide else 1) - 1, y + CH - 1], fill=bg)
        if sym.strip():
            font = font_for(sym)
            d.text((x, y + 1), sym, font=font, fill=fg, embedded_color=font is emoji)
    return img


def main(src, out):
    files = sorted(glob.glob(os.path.join(src, "frame_*.json")))
    frames = [frame(json.load(open(f, encoding="utf-8"))) for f in files]
    frames = [f.convert("P", palette=Image.ADAPTIVE, colors=64) for f in frames]
    frames[0].save(out, save_all=True, append_images=frames[1:], duration=200, loop=0, optimize=True)
    print(f"{out}: {len(frames)} frames, {os.path.getsize(out) // 1024} KB")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
