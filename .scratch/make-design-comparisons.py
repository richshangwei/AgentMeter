from pathlib import Path
from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parent.parent


def comparison(source: Path, implementation: Path, output: Path, size=(900, 540)):
    panel_width, panel_height = size
    header = 34
    canvas = Image.new("RGB", (panel_width * 2, panel_height + header), "#101827")
    draw = ImageDraw.Draw(canvas)
    draw.text((12, 10), "REFERENCE", fill="white")
    draw.text((panel_width + 12, 10), "IMPLEMENTATION", fill="white")
    for index, image_path in enumerate((source, implementation)):
        with Image.open(image_path).convert("RGB") as image:
            fitted = ImageOps.contain(image, (panel_width, panel_height))
            x = index * panel_width + (panel_width - fitted.width) // 2
            y = header + (panel_height - fitted.height) // 2
            canvas.paste(fitted, (x, y))
    canvas.save(output)


comparison(
    Path(r"C:\Users\richs\AppData\Local\Temp\codex-clipboard-aca3b5e1-63f2-4d17-bdaf-e6f4b5253d46.png"),
    ROOT / ".scratch" / "desktop-adaptive.png",
    ROOT / ".scratch" / "desktop-design-comparison.png",
)
comparison(
    Path(r"C:\Users\richs\AppData\Local\Temp\codex-clipboard-c331e70b-345f-41b1-aed6-8d6310a0161d.png"),
    ROOT / ".scratch" / "tablet-adaptive.png",
    ROOT / ".scratch" / "tablet-design-comparison.png",
)
