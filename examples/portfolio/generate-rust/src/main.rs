use std::fs;
use std::path::Path;
use std::thread;

fn copy_entry(src: &Path, dst: &Path) {
    if src.is_dir() {
        copy_dir(src, dst);
    } else {
        fs::copy(src, dst).unwrap();
    }
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let dest = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

fn main() {
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("examples/portfolio/data.json").unwrap()).unwrap();

    let template = fs::read_to_string("examples/portfolio/template.html").unwrap();
    let html = template.replace("{{PROJECTS_JSON}}", &data.to_string());

    fs::create_dir_all("dist-rust/images").unwrap();
    fs::write("dist-rust/index.html", html).unwrap();

    // Параллельное копирование
    let entries: Vec<_> = fs::read_dir("examples/portfolio/images")
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    let handles: Vec<_> = entries
        .into_iter()
        .map(|src| {
            let name = src.file_name().unwrap().to_owned();
            let dst = Path::new("dist-rust/images").join(&name);
            thread::spawn(move || copy_entry(&src, &dst))
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let count = data.as_array().map(|a| a.len()).unwrap_or(0);
    println!("Собрано: {} проектов → dist-rust/", count);
}
