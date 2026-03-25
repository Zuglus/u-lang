import json, shutil, os
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor

data = json.loads(Path("examples/portfolio/data.json").read_text())
template = Path("examples/portfolio/template.html").read_text()

html = template.replace("{{PROJECTS_JSON}}", json.dumps(data, ensure_ascii=False))

os.makedirs("dist-python/images", exist_ok=True)
Path("dist-python/index.html").write_text(html)

entries = list(Path("examples/portfolio/images").iterdir())

def copy_entry(e):
    dst = Path("dist-python/images") / e.name
    if e.is_dir():
        shutil.copytree(e, dst, dirs_exist_ok=True)
    else:
        shutil.copy2(e, dst)

with ThreadPoolExecutor() as pool:
    list(pool.map(copy_entry, entries))

print(f"Собрано: {len(data)} проектов → dist-python/")
