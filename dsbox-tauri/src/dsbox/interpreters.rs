use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Serialize)]
pub struct Found {
    pub language: Language,
    pub interpreter: String,
}

#[derive(Serialize, Copy, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Dart,
    JavaScript,
    Lua,
    Luau,
    Perl,
    Python,
    Ruby,
    Shell,
}

const EXTENSIONS: LazyLock<HashMap<&'static str, Language>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("dart", Language::Dart);
    m.insert("js", Language::JavaScript);
    m.insert("mjs", Language::JavaScript);
    m.insert("cjs", Language::JavaScript);
    m.insert("lua", Language::Lua);
    m.insert("luau", Language::Luau);
    m.insert("pl", Language::Perl);
    m.insert("pm", Language::Perl);
    m.insert("t", Language::Perl);
    m.insert("pod", Language::Perl);
    m.insert("py", Language::Python);
    m.insert("pyi", Language::Python);
    m.insert("pyc", Language::Python);
    m.insert("pyw", Language::Python);
    m.insert("pyx", Language::Python);
    m.insert("rb", Language::Ruby);
    m.insert("rake", Language::Ruby);
    m.insert("gemspec", Language::Ruby);
    m.insert("sh", Language::Shell);
    m.insert("bash", Language::Shell);
    m
});

const INTERPRETERS: LazyLock<HashMap<Language, &'static [&'static str]>> = LazyLock::new(|| {
    let mut m: HashMap<Language, &'static [&'static str]> = HashMap::new();
    m.insert(Language::Dart, &["dart"]);
    m.insert(Language::JavaScript, &["node"]);
    m.insert(Language::Lua, &["lua"]);
    m.insert(Language::Luau, &["luau"]);
    m.insert(Language::Perl, &["perl"]);
    m.insert(Language::Python, &["python", "python3"]);
    m.insert(Language::Ruby, &["ruby"]);
    m.insert(Language::Shell, &["bash", "sh"]);
    m
});

#[tauri::command]
pub fn find_interpreter(path: String) -> Option<Found> {
    let extension = std::path::Path::new(&path).extension()?;
    let language = *EXTENSIONS.get(extension.to_string_lossy().as_ref())?;
    let interpreters = INTERPRETERS;
    let interpreters = interpreters.get(&language)?;
    for interpreter in interpreters.iter() {
        if which::which(interpreter).is_ok() {
            return Some(Found {
                language,
                interpreter: interpreter.to_string(),
            });
        }
    }
    None
}
