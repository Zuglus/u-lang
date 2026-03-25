import os
from concurrent.futures import ProcessPoolExecutor

TEMPLATE = open("bench/template.html").read()

def process_bold(text):
    result = []
    rest = text
    while True:
        pos = rest.find("**")
        if pos == -1:
            result.append(rest)
            break
        result.append(rest[:pos])
        rest = rest[pos+2:]
        end = rest.find("**")
        if end == -1:
            result.append("**")
            result.append(rest)
            break
        result.append("<strong>")
        result.append(rest[:end])
        result.append("</strong>")
        rest = rest[end+2:]
    return "".join(result)

def process_links(text):
    result = []
    rest = text
    while True:
        pos = rest.find("[")
        if pos == -1:
            result.append(rest)
            break
        mid = rest.find("](", pos)
        if mid == -1:
            result.append(rest)
            break
        close = rest.find(")", mid + 2)
        if close == -1:
            result.append(rest)
            break
        result.append(rest[:pos])
        result.append(f'<a href="{rest[mid+2:close]}">{rest[pos+1:mid]}</a>')
        rest = rest[close+1:]
    return "".join(result)

def inline(text):
    return process_links(process_bold(text))

def process_file(filename):
    md = open(f"bench/content/{filename}").read()
    parts = []
    toc = []
    title = ""
    in_list = False

    for line in md.split("\n"):
        t = line.strip()
        if t.startswith("### "):
            if in_list: parts.append("</ul>"); in_list = False
            txt = t[4:]
            slug = txt.replace(" ", "-")
            toc.append(f'<li><a href="#{slug}">{txt}</a></li>')
            parts.append(f'<h3 id="{slug}">{inline(txt)}</h3>')
        elif t.startswith("## "):
            if in_list: parts.append("</ul>"); in_list = False
            txt = t[3:]
            slug = txt.replace(" ", "-")
            toc.append(f'<li><a href="#{slug}">{txt}</a></li>')
            parts.append(f'<h2 id="{slug}">{inline(txt)}</h2>')
        elif t.startswith("# "):
            if in_list: parts.append("</ul>"); in_list = False
            txt = t[2:]
            slug = txt.replace(" ", "-")
            if not title: title = txt
            toc.append(f'<li><a href="#{slug}">{txt}</a></li>')
            parts.append(f'<h1 id="{slug}">{inline(txt)}</h1>')
        elif t.startswith("- "):
            if not in_list: parts.append("<ul>"); in_list = True
            parts.append(f"<li>{inline(t[2:])}</li>")
        elif not t:
            if in_list: parts.append("</ul>"); in_list = False
        else:
            if in_list: parts.append("</ul>"); in_list = False
            parts.append(f"<p>{inline(t)}</p>")

    if in_list: parts.append("</ul>")

    page = TEMPLATE.replace("{{TITLE}}", title)
    page = page.replace("{{TOC}}", "".join(toc))
    page = page.replace("{{CONTENT}}", "".join(parts))

    with open(f"bench/output-python/{filename.replace('.md', '.html')}", "w") as f:
        f.write(page)

def main():
    os.makedirs("bench/output-python", exist_ok=True)
    files = sorted(f for f in os.listdir("bench/content") if f.endswith(".md"))
    with ProcessPoolExecutor() as pool:
        list(pool.map(process_file, files))
    print(f"Собрано: {len(files)} страниц \u2192 bench/output-python/")

if __name__ == "__main__":
    main()
