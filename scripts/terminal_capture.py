"""Render an observed pyte terminal buffer; requires optional Pillow and DejaVu."""
from pathlib import Path

def save_screen(screen, destination):
    from PIL import Image, ImageDraw, ImageFont
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    fonts = Path("/usr/share/fonts/truetype/dejavu")
    normal = ImageFont.truetype(str(fonts / "DejaVuSansMono.ttf"), 16)
    bold = ImageFont.truetype(str(fonts / "DejaVuSansMono-Bold.ttf"), 16)
    image = Image.new("RGB", (screen.columns * 10, screen.lines * 21), "#0f141d")
    draw = ImageDraw.Draw(image)
    named = {"default": "0f141d", "black": "000000", "white": "e0e7ef"}
    for y in range(screen.lines):
        for x in range(screen.columns):
            cell = screen.buffer[y][x]
            bg = named.get(cell.bg, cell.bg)
            fg = "e0e7ef" if cell.fg == "default" else named.get(cell.fg, cell.fg)
            draw.rectangle((x*10, y*21, x*10+10, y*21+21), fill="#"+bg)
            draw.text((x*10, y*21), cell.data, font=bold if cell.bold else normal, fill="#"+fg)
    image.save(destination.with_suffix(".png"))
    destination.with_suffix(".txt").write_text("\n".join(screen.display) + "\n")
