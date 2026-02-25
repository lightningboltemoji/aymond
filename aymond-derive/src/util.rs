pub fn to_ident_format(s: &str) -> String {
    s.chars()
        .map(|c| if c == '.' || c == '-' { '_' } else { c })
        .collect::<String>()
        .to_lowercase()
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect()
}
