use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

pub fn auto_generate_loader() {
    println!("cargo:rerun-if-env-changed=DEFT_JS_URL");
    println!("cargo:rerun-if-env-changed=DEFT_JS_DIR");
    let out_file = format!("{}/js_loader.code", env::var("OUT_DIR").unwrap());
    if let Ok(js_dir) = env::var("DEFT_JS_DIR") {
        generate_static_loader(js_dir.as_str(), out_file.as_str());
    } else {
        let js_url = env::var("DEFT_JS_URL").unwrap_or("http://localhost:7800".to_string());
        generate_dev_loader(js_url.as_str(), out_file.as_str());
    }
}

pub fn generate_dev_loader(url: &str, output_dir: &str) {
    let code = format!("Box::new(deft::loader::DevModuleLoader::new(Some(\"{}\")))", url);
    write_code(code.as_str(), output_dir);
}

pub fn generate_static_loader(js_dir: &str, output_dir: &str) {
    let path = PathBuf::from_str(&js_dir).unwrap();
    let canonical_path = path.canonicalize().unwrap();
    let files = path.read_dir().unwrap();

    let mut code = String::new();
    code.push_str("{\n");
    code.push_str("let mut loader = deft::loader::StaticModuleLoader::new();\n");
    for f in files {
        let file_name = f.unwrap().file_name();
        let full_path = canonical_path.join(&file_name).to_string_lossy().to_string().replace("\\", "\\\\");
        let name = file_name.to_str().unwrap();
        code.push_str(&format!("loader.add_module(\"{}\".to_string(), include_str!(\"{}\").to_string());\n", name, full_path));
    }
    code.push_str("Box::new(loader)\n");
    code.push_str("}");
    write_code(code.as_str(), output_dir);
}

fn write_code(code: &str, output_dir: &str) {
    let mut file = File::create(output_dir).unwrap();
    file.write_all(code.as_bytes()).unwrap();
}