fn main() {
    let lines = [
        "#include \"jccolext.c\"",
        "  #include \"jcgryext.c\"",
        "#include \"jinclude.h\"",
        "/* jccolext.c */",
    ];
    let mut set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in lines.iter() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#include") {
            let rest = rest.trim_start();
            if rest.starts_with('"') {
                if let Some(end) = rest[1..].find('"') {
                    let target = &rest[1..end];
                    if target.ends_with(".c") {
                        println!("MATCH target={:?}", target);
                        set.insert(target.to_string());
                    }
                }
            }
        }
    }
    println!("set={:?}", set);
}
