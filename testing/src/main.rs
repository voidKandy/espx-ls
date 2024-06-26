use regex::Regex;
use std::io::{self, Read};

// ⚑

fn main() {
    let mut raw = String::new();
    io::stdin()
        .read_to_string(&mut raw)
        .expect("failed to read io");
    let re = Regex::new(r#""stderr"\s+'([^']+)'"#).unwrap();
    for entry in raw.split(r#"\n{"#).map(|s| {
        if s.starts_with('{') {
            s.to_string()
        } else {
            format!("{{{}", s)
        }
    }) {
        let cleaned_entry = entry.replace(r#"\\"#, r#"\"#);

        if let Some(captures) = re.captures(&cleaned_entry) {
            if let Some(stderr_part) = captures.get(1) {
                let sani = stderr_part.as_str().replace(r#"\\n"#, "\n");
                let sani = sani.as_str().trim_end_matches("\\n");
                println!("{}\n", sani);
            }
        }
    }
}
