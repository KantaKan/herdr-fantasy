# herdr fantasy — Art Bible

All characters are **original**. Do not draw or reference vendor logos, mascots, or brand
colors of Anthropic, OpenAI, Cursor, Google, or any agent vendor. Agents are identified by
their name as text; the class is the costume.

## 1. Style
- Chunky, readable pixel art. Think GBA-era RPG, 1px dark outline on everything.
- Big heads, small bodies (chibi, ~1:1 head-to-body) so faces read at tiny sizes.
- Every sprite must look good at **halfblock resolution** (1 px = half a terminal cell). Test there first; Kitty only makes it crisper.
- Light comes from the top-left. One highlight color, one shadow color per material.

## 2. Palette — Sweetie 16 (use only these)

| Key | Hex | Use |
|---|---|---|
| `k` | `#1a1c2c` | outline, darkest |
| `p` | `#5d275d` | deep shadow, magic |
| `r` | `#b13e53` | red, damage, Knight plume |
| `o` | `#ef7d57` | skin shadow, fire |
| `y` | `#ffcd75` | skin, gold, XP |
| `G` | `#a7f070` | bright green, heal, slime highlight |
| `g` | `#38b764` | green, slime |
| `t` | `#257179` | teal shadow |
| `n` | `#29366f` | navy, night sky |
| `b` | `#3b5dc9` | blue, Mage robe |
| `c` | `#41a6f6` | cyan, mana, stamina bar |
| `C` | `#73eff7` | ice, sparkle |
| `w` | `#f4f4f4` | white, hit flash, eyes |
| `s` | `#94b0c2` | steel light |
| `S` | `#566c86` | steel dark, stone |
| `d` | `#333c57` | dark gray, UI panels |
| `.` | — | transparent |

UI colors: panel bg `d`, panel border `S`, text `w`, dim text `s`, XP bar `y`, stamina bar `c` (turns `o` at Tired, `r` at Exhausted), HP bar `r`.

## 3. Sizes (pixels; halfblock cells = width × height/2)

| Asset | Size | Cells (halfblock) |
|---|---|---|
| Hero (battle) | 16×16 | 16×8 |
| Hero face (compact mode) | 8×8 | 8×4 |
| Portrait (ask popup, char sheet) | 32×32 | 32×16 |
| Summon (Joke Class) | 12×12 | 12×6 |
| Rubber Duck | 8×8 | 8×4 |
| Slime / small mob | 10×8 | 10×4 |
| Boss — little / normal / big | 14×12 / 24×20 / 34×28 | 14×6 / 24×10 / 34×14 |
| Chest, campfire, props | 8×8 | 8×4 |
| Town building tiers | 16×16, 20×20, 24×24 | — |
| Digits font | 3×5 per glyph | — |

## 4. Classes (heroes)

| Class | Default agent | Look |
|---|---|---|
| **Knight** | claude | rounded steel helmet (`s`/`S`) with a red plume (`r`), blue tunic, short sword. Visor slit shows two white eye dots. |
| **Mage** | codex | tall floppy hat and robe in `b`/`n` with gold trim, staff with a cyan orb. |
| **Rogue** | cursor | dark hood (`d`/`p`), scarf, twin daggers, eyes glow `C`. |
| **Ranger** | opencode | green cloak (`g`/`t`), bow, leaf on hood. |
| **Cleric** | gemini | white/gold robe (`w`/`y`), round shield with a sun. |
| **Bard** | any other | orange vest (`o`), feathered cap, lute. |
| **Sahur Knight** (special, opt-in via `[classes]`) | — | its own body, not the shared template: a proper knight whose **helmet is a wooden kentongan drum** (`o`/`r` with grain, `y` rim, the drum slit is the visor with glowing `y` eyes — `r` when angry), red plume, `s`/`S` steel chest with a gold emblem and wooden belt, red cape that flutters when running, steel legs, and a big wooden `y` bat: upright at rest, wound up / swung across / followed through on attacks, raised high for spells and victory. Face and portrait are the drum helmet head-on. Drawn by `tools/gen_special.py`. Original design — not the "Tung Tung Tung Sahur" meme character. |
| **Wild Card** (special, opt-in) | — | an original harlequin trickster — the joker of a card deck, not any comic character: two-pointed `r`/`y` jester hat with bouncing bells, white `w`/`s` mask with a blue diamond tear and a sly `r` grin, ruffled collar, `r`/`y` diamond motley with pom-pom buttons, one red and one yellow leg, curly pointed shoes. Holds a fan of cards; attacks wind up and flick a card; juggles three balls for spells; bows, then jumps for victory; sits cross-legged at camp. Deliberately avoids the DC Joker's trademarks (green hair, purple suit, scarred smile). |

Each class needs these animations (frames @ ms per frame):

| Anim | Frames | ms | Notes |
|---|---|---|---|
| `idle` | 4 | 250 | breathing, 1px bob |
| `sit` | 2 | 600 | at campfire (idle state) |
| `walk` | 4 | 100 | run cycle: head/shoulders lean 1px forward, wide stride with arms swinging opposite, body lifts 1px on the passing frames |
| `attack` | 3 | 80 | wind-up, swing, follow-through |
| `cast` | 4 | 100 | used for Bash |
| `open` | 4 | 120 | opening a chest |
| `scout` | 2 | 300 | spyglass (web) |
| `hurt` | 2 | 100 | boss hits back |
| `raise_hand` | 2 | 300 | blocked |
| `victory` | 4 | 150 | |
| `tired` | 2 | 300 | sweat drop |
| `exhausted` | 2 | 450 | panting, slumped |
| `rest` | 2 | 700 | sitting asleep by fire |
| `talk` (portrait) | 2 | 180 | mouth open/closed |

## 4b. Joke Classes (Summons)
Worn only by Summons. 12×12, same chibi rules. Original archetypes only — never copy a real character's costume, emblem, or colors.

| Joke Class | Look |
|---|---|
| **10x Engineer** | stack of tiny headphones (`d`/`c`), hoodie `n`, keyboard in each hand |
| **Jester** | `p`/`y` diamond motley, belled cap, wild grin |
| **Night Owl** | `n`/`k` cape and cowl with owl ears, glowing `y` eyes, mug |
| **Intern** | cardboard crown (`o`), lanyard, coffee tray |
| **Stack Overflow Pilgrim** | robe `S`, giant unrolling scroll `w` |
| **Vibe Coder** | sunglasses `k`, headphones `r`, loose shirt `C` |
| **Senior Dev** | grey beard `s`, cardigan `t`, mug |
| **YAML Wizard** | robe of stepped indentation lines `b`/`c` |
| **Product Manager** | blazer `d`, rolled roadmap pointer `y` |
| **On-Call Paladin** | armor `s` with tiny pager `r` blinks, droopy eyes |
| **Rubber Duck Cultist** | hooded robe `y` with duck emblem `o` |
| **Merge Conflict Twins** | one sprite split down the middle, left `G`, right `r` |

Summon anims: `rise` 4f @ 120 ms (`p`/`k` shadow grows from the ground, then costume fades in), `idle` 2f, `attack` 3f, `sink` 4f (reverse of rise).

## 5. Enemies and props
- **Slime** (`g`/`G`/`t`): `idle` 2f, `hurt` 1f (white flash frame), `die` 4f (splat into droplets).
- **Bosses** (SPEC §6.3), each in 3 size tiers. Anims: `idle` 2f, `attack` 3f, `hurt` 2f, `laugh` 2f (on failure), `die` 6f (placeholders: idle/laugh/die, 2f each).
  - Test: **Test Golem** (stone `S`/`s`, glowing `y` core and eyes that turn `r` when laughing) · **Flaky Ghost** (`w`/`s` sheet, wavy hem, `C` sparkles) · **Snapshot Mimic** (`o`/`r` chest with `w` teeth that opens its lid to laugh).
  - Build: **Build Wyrm** (`r`/`p` coiled dragon, `y` horns and gear dots, breathes `o` fire) · **Linker Kraken** (`r` head, `p` tentacles) · **Dependency Hydra** (three `G`/`g` heads on one `g`/`t` body).
  - Lint: **Lint Imp** (`o` imp, `r` horns, red pen) · **Typo Gremlin** (`g` with big `G` ears and a `w` grin) · **Off-by-One Slime King** (giant slime with a `y` crown).
  - Rare: **Sahur Knocker** (`o`/`r` wooden log, `y` cut top, big `w` eyes, raised `y` bat it swings when laughing). Original design — never copy the "Tung Tung Tung Sahur" meme character.
  - Placeholders are generated by `tools/gen_bosses.py` (shapes in normalized coords → every tier, auto `k` outline).
- **Rubber Duck** (`y` body, `o` beak, `k` outline): `idle` 2f, `quack` 2f (beak open + speech bubble), `waddle` 4f.
- Props: chest (closed/open), campfire 3f, `❗` bubble 2f, `z` 2f, sparkle 3f, rune circle 4f, journal (rest scene), level-up light column 4f.

## 6. Ask popup (JRPG box)
- Double-line box drawn with box characters in halfblock mode; in Kitty mode a 9-slice panel image (8×8 corners) in `d` with `S` border and a 1px `w` inner highlight on the top-left edges.
- Portrait framed left (32×32 + 1 cell padding). Name plate above the text in the hero's class color.
- Selection cursor `▶` blinks at 500 ms.

## 7. Town
- Building = top-level folder. Three size tiers. Style follows the theme:
  - Forge Village (Rust): stone + orange glowing forges.
  - Lantern City (TS/JS): wooden houses with hanging `y` lanterns.
  - Harbor Town (Go): blue roofs, a pier.
  - Serpent Grove (Python): tree houses.
  - Meadow Hamlet (other): thatched cottages.
- Decorations unlocked by commit count: 10 tree, 50 well, 100 lanterns, 250 windmill, 500 statue of the top hero, 1000 castle on the hill.

## 8. Asset format and pipeline
- Source: Aseprite files in `art/` (not shipped), exported to `assets/sprites/<name>.png` + `<name>.json` (Aseprite "Array" JSON with frame tags = animation names, frame durations in ms).
- Until real art exists, generate **placeholders** from text pixel maps (`assets/placeholders/*.txt`) using the palette keys above, one frame per block separated by a blank line. A build script converts them to PNG + JSON in the same format as Aseprite exports so the runtime never knows the difference.

Example placeholder, `slime_idle` frame 1 (10×8):

```
...kkkk...
..kgGGgk..
.kgGgggggk
kgwkggwkgk
kgggggggdk
kggggggtdk
.kttttttdk
..kkkkkkk.
```
(`d` = drop shadow, `w`/`k` pairs = eyes.)

Placeholder rule for heroes: 16×16 silhouette in the class's main color with `k` outline, `w` eye dots, and a 2px accessory in the accent color (plume, hat tip, hood, leaf, sun, feather) so classes are distinguishable even before real art.
