use std::io::{self, Read};

// @_ hello
fn main() {
    let mut raw = String::new();
    io::stdin()
        .read_to_string(&mut raw)
        .expect("failed to read io");

    // let prefix = r#""stderr""#;
    // for entry in raw.split("\n") {
    //     if let Some(idx) = entry.find(prefix) {
    //         let chunk = entry[idx + prefix.len()..].to_string();
    //         let sani = chunk.as_str().replace(r#"\\n"#, "\n").trim().to_string();
    //         let sani = sani.trim_end_matches("\\n\'");
    //         let sani = sani.trim_end_matches("\\n\"");
    //         let sani = sani.trim_start_matches("\'");
    //         let sani = sani.trim_start_matches("\"");
    //         let sani = sani.replace(r#"\\"#, r#"\"#);
    //         let sani = sani.replace(r#"\""#, r#"""#);
    //         for mut s in sani.split(r#"}\n{"v"#).map(|s| s.to_string()) {
    //             let slice: String = s.chars().take(3).collect();
    //             if slice == r#"{"v"# {
    //                 s = format!("{}", s);
    //             } else {
    //                 s = format!("{{\"v{}\n", s);
    //             }
    //             match serde_json::from_str::<serde_json::Value>(&s) {
    //                 Ok(json_value) => {
    //                     println!("{}", json_value);
    //                 }
    //                 Err(_) => {
    //                     println!("{}\n", s);
    //                 }
    //             }
    //         }
    //     }
    // }
}

struct ToBePushed;
