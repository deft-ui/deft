use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

pub fn auto_generate_loader() {
    println!("cargo:rerun-if-env-changed=DEFT_JS_DIR");
    if let Ok(js_dir) = env::var("DEFT_JS_DIR") {
        generate_static_loader(js_dir.as_str(), "target/deft_js_loader.in");
    } else {
        let js_url = env::var("DEFT_JS_URL").unwrap_or("http://localhost:7800".to_string());
        generate_dev_loader(js_url.as_str(), "target/deft_js_loader.in");
    }
}

pub fn generate_dev_loader(url: &str, output_dir: &str) {
    let mut code = format!("Box::new(deft::loader::DevModuleLoader::new(Some(\"{}\")))", url);
    write_code(code.as_str(), output_dir);
}

pub fn generate_static_loader(js_dir: &str, output_dir: &str) {
    let path = PathBuf::from_str(&js_dir).unwrap();
    let canonical_path = path.canonicalize().unwrap().to_string_lossy().to_string();
    let files = path.read_dir().unwrap();

    let mut code = String::new();
    code.push_str("{\n");
    code.push_str("let mut loader = deft::loader::StaticModuleLoader::new();\n");
    for f in files {
        let file_name = f.unwrap().file_name();
        let name = file_name.to_str().unwrap();
        code.push_str(&format!("loader.add_module(\"{}\".to_string(), include_str!(\"{}/{}\").to_string());\n", name, canonical_path, name));
    }
    code.push_str("Box::new(loader)\n");
    code.push_str("}");
    write_code(code.as_str(), output_dir);
}

fn write_code(code: &str, output_dir: &str) {
    let mut file = File::create(output_dir).unwrap();
    file.write_all(code.as_bytes()).unwrap();
}