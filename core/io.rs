// Ядро: ввод-вывод — Rust сторона.
// Тонкая обёртка над println!. .u файл io.u подключает это через `use rust_io`.

pub fn println_raw(s: String) {
    println!("{}", s);
}
