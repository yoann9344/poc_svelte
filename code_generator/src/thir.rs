use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    io::Read,
    path::Path,
    process::{Command, Output},
    str::Chars,
};

const DIR_THIR_ARTEFACTS: &str = "target/thir/";

fn generate_rustc_thir<P: AsRef<Path>>(path: P) -> Output {
    Command::new("rustc")
        .arg("-Z")
        .arg("unpretty=thir-flat")
        .arg("--color")
        .arg("always")
        .arg("--allow")
        .arg("unused_variables")
        .arg(path.as_ref().to_str().expect("Unvalid Path to str."))
        .output()
        .expect("failed to execute process")
}

fn write_code_and_thir(path_code: &Path, out: &Path, err: &Path, code: &str) -> (String, String) {
    fs::write(path_code, code).expect("Can't write CODE.");
    let output = &generate_rustc_thir(path_code);
    let stdout = String::from_utf8(output.stdout.to_owned()).expect("Can't parse THIR utf-8.");
    let stderr = String::from_utf8(output.stderr.to_owned()).expect("Can't parse Stderr as utf-8.");
    fs::write(out, stdout.clone()).expect("Can't write THIR.");
    fs::write(err, stderr.clone()).expect("Can't write THIR.");

    (stdout, stderr)
}

fn generate_thir(code: &str, file_name: OsString, code_name: String) -> String {
    let path_thir_artefacts: &Path = Path::new(DIR_THIR_ARTEFACTS);
    let path_file = &path_thir_artefacts.join(file_name);
    let path_code = &path_file.join(format!("{code_name}.rs"));
    let path_info = &path_file.join(format!("{code_name}.txt"));
    let path_errors = &path_file.join(format!("{code_name}.stderr"));
    fs::create_dir_all(path_file).expect("Can't create THIR directory.");

    let (stdout, stderr) = match fs::File::open(path_code) {
        Ok(mut file_info) => {
            let mut content = String::new();
            file_info
                .read_to_string(&mut content)
                .expect("Can't read CODE file.");
            if code != content {
                write_code_and_thir(path_code, path_info, path_errors, code)
            } else {
                (
                    fs::read_to_string(path_info).expect("Can't read THIR file."),
                    fs::read_to_string(path_errors).expect("Can't read THIR file."),
                )
            }
        }
        Err(_) => write_code_and_thir(path_code, path_info, path_errors, code),
    };
    println!("{}\n\n{}", stdout, stderr);
    stdout
}

// struct Expr {
//     kind: ...,
//     ty: ...,
//     temp_lifetime: Option<Node>,
//     span: Span,
// }

// struct Block {
//     targeted_by_break: bool,
//     region_scope: Node,
//     opt_destruction_scope: Option<Destruction>,
//     span: Span,
//     stmts: Vec<Stmt>,
//     expr: Option<Id>,
//     safety_mode: Safe,
// }

#[derive(Debug, Default)]
struct Location {
    line: u32,
    column: u32,
}

impl Location {
    fn from_iterator<'a, I>(iterator: &mut I) -> Option<Self>
    where
        I: Iterator<Item = u32> + 'a,
    {
        Some(Location {
            line: iterator.next()?,
            column: iterator.next()?,
        })
    }
}

#[derive(Debug)]
struct Span {
    file: String,
    start: Location,
    end: Location,
}

impl From<String> for Span {
    /// target/thir/src/main.rs/for_loop_1.rs:6:9:6:13(#0)
    fn from(value: String) -> Self {
        let mut file = String::new();
        let mut positions = Vec::new();

        let mut buf = String::new();
        let mut file_parsed = false;
        for c in value.chars() {
            match c {
                ':' => {
                    if file_parsed {
                        positions.push(buf.clone());
                        // match str::parse(buf.as_str()) {
                        //     Ok(num) => positions.push(num),
                        //     Err(_) => positions.push(0),
                        // }
                    } else {
                        file = buf.clone();
                        file_parsed = true;
                    }
                    buf.clear();
                }
                '(' if file_parsed => break,
                _ => buf.push(c),
            }
        }
        positions.push(buf.clone());
        let positions: Vec<u32> = positions.iter().map(|s| s.parse().unwrap_or(0)).collect();
        let mut iter_position = positions.iter().cloned();
        let start = Location::from_iterator(&mut iter_position).unwrap_or_default();
        let end = Location::from_iterator(&mut iter_position).unwrap_or_default();
        Self { file, start, end }
    }
}

#[derive(Debug)]
pub struct PatBinding {
    pub name: String,
    pub ty: String,
    // pub span: Span,
    // mutability: Mutability,
    // mode: Mode,
}

#[derive(Default)]
struct RawInfo {
    bindings: Vec<PatBinding>,
    thir: Thir,
    dbg_line: usize,
    // exprs: Vec<Expr>,
}

#[derive(Debug)]
enum Thir {
    List(Vec<Thir>),
    Fields((String, HashMap<String, Thir>)),
    NamedList((String, Vec<Thir>)),
    Text(String),
}

impl Default for Thir {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

macro_rules! instance_from_thir_raw_string {
    ( $struct:ident: $map:ident => $( $name:ident ),* ) => {
        {
            $(
                let $name = match $map.get(&stringify!($name).to_string()).unwrap() {
                    Thir::Text(string) => string.to_owned(),
                    _ => return,
                };
            )*
            $struct { $(
                $name: $name.into(),
                )*
            }
        }
    };
}
fn remove_last_char(buf: String) -> String {
    let mut chars = buf.chars();
    chars.next_back();
    chars.collect()
}

impl RawInfo {
    fn serialize(thir_raw: String) -> Result<Self, String> {
        let mut s = Self::default();
        let mut chars = thir_raw.chars();
        for c in &mut chars {
            if c == '\n' {
                break;
            }
        }

        s.thir = s.switch(&mut chars)?;
        Ok(s)
    }

    fn switch(&mut self, thir_raw: &mut Chars) -> Result<Thir, String> {
        let mut buf = String::new();
        let mut prev_c = ' ';
        loop {
            let c = match thir_raw.next() {
                Some(next) => next,
                None => Err("Triage as consumed whole chars.")?,
            };
            match c {
                '\n' => {
                    self.dbg_line += 1;
                    match prev_c {
                        '{' => {
                            let name = remove_last_char(buf);
                            let thir = Thir::Fields((name, self.parse_map(thir_raw)));
                            self.insert_item(&thir);
                            return Ok(thir);
                        }
                        '[' => {
                            let thir = Thir::List(self.parse_vec(thir_raw));
                            self.insert_item(&thir);
                            return Ok(thir);
                        }
                        '(' => {
                            let name = remove_last_char(buf);
                            return Ok(Thir::NamedList((name, self.parse_vec(thir_raw))));
                        }
                        ',' => {
                            if buf.len() == 2 && "}])".contains(buf.chars().next().unwrap()) {
                                return Err("Close delimiter.".to_string());
                            } else {
                                let mut chars = buf.chars();
                                // remove last coma
                                chars.next_back();
                                let text = match (chars.next(), chars.next_back()) {
                                    // remove double quote
                                    (Some('"'), Some('"')) => chars.as_str().to_string(),
                                    _ => remove_last_char(buf),
                                };
                                return Ok(Thir::Text(text));
                            }
                        }
                        _ => panic!(
                            "Triage : line must end with {{ or [ or ( but ends with '{}'",
                            prev_c
                        ),
                    }
                }
                // chars that should not be pushed
                ' ' => (),
                _ => buf.push(c),
            }
            prev_c = c;
        }
    }

    fn parse_vec(&mut self, thir_raw: &mut Chars) -> Vec<Thir> {
        let mut thirs = Vec::new();
        loop {
            let thir_raw_option = self.switch(thir_raw);
            match thir_raw_option {
                Ok(thir) => thirs.push(thir),
                Err(_) => break,
            }
        }
        thirs
    }

    fn parse_map(&mut self, thir_raw: &mut Chars) -> HashMap<String, Thir> {
        let mut tree = HashMap::new();
        let mut name = String::new();
        loop {
            let c = match thir_raw.next() {
                Some(next) => next,
                None => break,
            };
            match c {
                '}' => {
                    break;
                }
                ':' => match self.switch(thir_raw) {
                    Ok(thir) => {
                        tree.insert(name.clone(), thir);
                        name.clear();
                    }
                    Err(_) => break,
                },
                ' ' => (),
                '\n' => self.dbg_line += 1,
                ',' => (),
                // it seems to not contains neither numbers or uppercase...
                'a'..='z' | '_' => name.push(c),
                c => panic!(
                    "Unknown char in Brace item : '{}' at line {}",
                    c, self.dbg_line
                ),
            }
        }
        tree
    }

    fn insert_item(&mut self, thir: &Thir) {
        match thir {
            Thir::Fields((name, pat_fields)) if name == "Pat" => match pat_fields.get("kind") {
                Some(Thir::Fields((name, binding_fields))) if name == "Binding" => {
                    println!("INSERT : {:#?}", thir);
                    let mut fields = HashMap::new();
                    fields.extend(pat_fields);
                    fields.extend(binding_fields);
                    // let binding =
                    //     instance_from_thir_raw_string!(PatBinding: fields => name, ty, span);
                    let binding = instance_from_thir_raw_string!(PatBinding: fields => name, ty);
                    self.bindings.push(binding);
                }
                _ => (),
            },
            _ => (),
        }
    }
}

fn format_main_code(code: String) -> String {
    format!(
        r#"
        fn main() {{
            {code}
        }}
        "#
    )
}

fn get_pat_bindings_wrap(
    code: String,
    file: OsString,
    code_name: String,
) -> Result<Vec<PatBinding>, String> {
    let main_code = format_main_code(code);
    let thir_raw = generate_thir(main_code.as_str(), file, code_name);
    let raw_info = RawInfo::serialize(thir_raw)?;
    Ok(raw_info.bindings)
}

pub fn get_pat_bindings(code: String, file: OsString, code_name: String) -> Vec<PatBinding> {
    match get_pat_bindings_wrap(code, file.clone(), code_name.clone()) {
        Ok(bindings) => bindings,
        Err(error) => {
            println!("{}", error);
            panic!(
                "Failed to parse THIR {code_name} in {}.",
                file.to_str().unwrap_or("{unknown}")
            );
        }
    }
}
