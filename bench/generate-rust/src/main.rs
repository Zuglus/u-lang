use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

fn process_bold(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    loop {
        let Some(pos) = rest.find("**") else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..pos]);
        rest = &rest[pos + 2..];
        let Some(end) = rest.find("**") else {
            result.push_str("**");
            result.push_str(rest);
            break;
        };
        result.push_str("<strong>");
        result.push_str(&rest[..end]);
        result.push_str("</strong>");
        rest = &rest[end + 2..];
    }
    result
}

fn process_links(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    loop {
        let Some(pos) = rest.find('[') else {
            result.push_str(rest);
            break;
        };
        let Some(mid_rel) = rest[pos..].find("](") else {
            result.push_str(rest);
            break;
        };
        let mid = pos + mid_rel;
        let Some(close_rel) = rest[mid + 2..].find(')') else {
            result.push_str(rest);
            break;
        };
        let close = mid + 2 + close_rel;
        result.push_str(&rest[..pos]);
        let link_text = &rest[pos + 1..mid];
        let url = &rest[mid + 2..close];
        result.push_str("<a href=\"");
        result.push_str(url);
        result.push_str("\">");
        result.push_str(link_text);
        result.push_str("</a>");
        rest = &rest[close + 1..];
    }
    result
}

fn inline(text: &str) -> String {
    process_links(&process_bold(text))
}

fn process_file(src: &Path, dst: &Path, template: &str) {
    let md = fs::read_to_string(src).unwrap();

    let mut html = String::new();
    let mut toc = String::new();
    let mut title = String::new();
    let mut in_list = false;

    for line in md.lines() {
        let t = line.trim();
        if t.starts_with("### ") {
            if in_list { html.push_str("</ul>"); in_list = false; }
            let txt = &t[4..];
            let slug = txt.replace(' ', "-");
            toc.push_str("<li><a href=\"#");
            toc.push_str(&slug);
            toc.push_str("\">");
            toc.push_str(txt);
            toc.push_str("</a></li>");
            html.push_str("<h3 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&inline(txt));
            html.push_str("</h3>");
        } else if t.starts_with("## ") {
            if in_list { html.push_str("</ul>"); in_list = false; }
            let txt = &t[3..];
            let slug = txt.replace(' ', "-");
            toc.push_str("<li><a href=\"#");
            toc.push_str(&slug);
            toc.push_str("\">");
            toc.push_str(txt);
            toc.push_str("</a></li>");
            html.push_str("<h2 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&inline(txt));
            html.push_str("</h2>");
        } else if t.starts_with("# ") {
            if in_list { html.push_str("</ul>"); in_list = false; }
            let txt = &t[2..];
            let slug = txt.replace(' ', "-");
            if title.is_empty() { title = txt.to_string(); }
            toc.push_str("<li><a href=\"#");
            toc.push_str(&slug);
            toc.push_str("\">");
            toc.push_str(txt);
            toc.push_str("</a></li>");
            html.push_str("<h1 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&inline(txt));
            html.push_str("</h1>");
        } else if t.starts_with("- ") {
            if !in_list { html.push_str("<ul>"); in_list = true; }
            html.push_str("<li>");
            html.push_str(&inline(&t[2..]));
            html.push_str("</li>");
        } else if t.is_empty() {
            if in_list { html.push_str("</ul>"); in_list = false; }
        } else {
            if in_list { html.push_str("</ul>"); in_list = false; }
            html.push_str("<p>");
            html.push_str(&inline(t));
            html.push_str("</p>");
        }
    }

    if in_list { html.push_str("</ul>"); }

    let page = template
        .replace("{{TITLE}}", &title)
        .replace("{{TOC}}", &toc)
        .replace("{{CONTENT}}", &html);

    fs::write(dst, page).unwrap();
}

fn main() {
    let template = Arc::new(fs::read_to_string("bench/template.html").unwrap());

    fs::create_dir_all("bench/output-rust").unwrap();

    let mut entries: Vec<_> = fs::read_dir("bench/content")
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .map(|e| e.path())
        .collect();
    entries.sort();

    let handles: Vec<_> = entries
        .iter()
        .map(|src| {
            let src = src.clone();
            let tpl = template.clone();
            let name = src
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".md", ".html");
            let dst = Path::new("bench/output-rust").join(&name);
            thread::spawn(move || process_file(&src, &dst, &tpl))
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    println!("Собрано: {} страниц \u{2192} bench/output-rust/", entries.len());
}
