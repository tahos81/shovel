#![allow(unused)]
#[path = "../file_storage/svg_to_png.rs"]
mod svg_to_png;

fn main() {
    let file_path = std::env::args().nth(1).expect("convert.rs /path/to/svg /path/to/png");
    let target_path = std::env::args().nth(2).expect("convert.rs /path/to/svg /path/to/png");

    println!("{} {}", file_path, target_path);

    let mut svg_bytes = std::fs::read(file_path).expect("Couldn't read file");

    match svg_to_png::svg_to_png(&svg_bytes[..]) {
        Ok(data) => {
            let png_file = std::fs::write(target_path, data).expect("Failed to write to png");
        }
        Err(e) => {
            dbg!(format!("{:?}", e));
        }
    };
}
