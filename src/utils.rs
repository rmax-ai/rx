pub fn sanitize_goal_slug(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;

    for ch in input.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_was_dash = false;
            continue;
        }

        if (ch.is_whitespace() || ch == '-' || ch == '_' || ch == '/')
            && !slug.is_empty()
            && !previous_was_dash
        {
            slug.push('-');
            previous_was_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    const MAX_SLUG_CHARS: usize = 48;
    if slug.len() > MAX_SLUG_CHARS {
        slug.truncate(MAX_SLUG_CHARS);
        while slug.ends_with('-') {
            slug.pop();
        }
    }

    if slug.is_empty() {
        "goal".to_string()
    } else {
        slug
    }
}
