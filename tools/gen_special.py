"""Run: python tools/gen_special.py

Special hero classes built from the shared templates (hero/face/portrait.txt):
  Sahur Knight — a knight whose helmet is a wooden kentongan drum (the visor slit is the
  drum's slit), wood-grain armor, a yellow cut-top crest, and a wooden bat for a sword.
Original design, inspired by the sahur wake-up drum."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PH = os.path.join(HERE, "..", "assets", "placeholders")

SPECIALS = {
    # name: (main, shadow, accent, grain, weapon)
    "sahur_knight": ("o", "r", "y", "r", "y"),
}


def frames_of(text):
    """Yield (line) with tag lines kept, so the file structure is preserved."""
    return text.split("\n")


def recolor(text, main, shadow, accent, grain, weapon):
    out = []
    y = 0
    for line in frames_of(text):
        if line.startswith("#") or not line.strip():
            out.append(line)
            y = 0
            continue
        row = []
        for x, ch in enumerate(line):
            if ch == "M":
                # wood grain: thin darker streaks running across the planks
                ch = grain if (x * 3 + y) % 7 == 0 else main
            elif ch == "D":
                ch = shadow
            elif ch == "A":
                ch = accent
            elif ch == "s":
                ch = weapon  # the sword becomes a wooden bat
            row.append(ch)
        out.append("".join(row))
        y += 1
    return "\n".join(out)


for name, colors in SPECIALS.items():
    for stem in ("hero", "face", "portrait"):
        src = open(os.path.join(PH, f"{stem}.txt")).read()
        dst = os.path.join(PH, f"{stem}_{name}.txt")
        open(dst, "w", newline="\n").write(recolor(src, *colors))
print("ok")
