use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use zip::write::{FileOptions, ZipWriter};

fn main() {
    // Get the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let parser_class_dir = out_dir.join("parser-classes");

    // Compile the Java parser code
    let _build = javac::Build::new()
        .source_dir("src/java")
        .output_dir(&parser_class_dir)
        .release("11") // Java 11 for compiler API access
        .compile();

    // Create JAR file from compiled parser classes
    let parser_jar = out_dir.join("jbindgen-parser.jar");
    create_jar(&parser_class_dir, &parser_jar);

    println!(
        "cargo:rustc-env=JBINDGEN_PARSER_CLASSES={}",
        parser_class_dir.display()
    );

    // Store JAR file path for embedding
    println!(
        "cargo:rustc-env=JBINDGEN_PARSER_JAR={}",
        parser_jar.display()
    );

    // Rerun if Java source files change
    println!("cargo:rerun-if-changed=src/java");
}

fn create_jar(class_dir: &PathBuf, jar_path: &PathBuf) {
    let file = File::create(jar_path).expect("Failed to create JAR file");
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);

    add_directory_to_jar(&mut zip, class_dir, class_dir, options);

    zip.finish().expect("Failed to finish JAR file");
}

fn add_directory_to_jar(
    zip: &mut ZipWriter<File>,
    base_dir: &PathBuf,
    current_dir: &PathBuf,
    options: FileOptions<()>,
) {
    use std::fs;

    let entries = fs::read_dir(current_dir).expect("Failed to read directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_dir() {
            add_directory_to_jar(zip, base_dir, &path, options);
        } else if path.is_file() {
            let relative_path = path
                .strip_prefix(base_dir)
                .expect("Failed to get relative path");
            let zip_path = relative_path.to_str().expect("Invalid UTF-8 in path");

            zip.start_file(zip_path, options)
                .expect("Failed to start file in JAR");

            let contents = fs::read(&path).expect("Failed to read class file");
            zip.write_all(&contents)
                .expect("Failed to write class file to JAR");
        }
    }
}
