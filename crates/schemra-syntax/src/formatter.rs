/// Normalize line endings and indentation while preserving comments.
///
/// Formatting is deliberately best-effort so it also works while a user is editing
/// incomplete source. Schemra uses two spaces per block level and one trailing newline.
pub fn format_source(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let Some(first_content) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return String::new();
    };
    let last_content = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .expect("the first content line exists");

    let mut depth = 0usize;
    let mut formatted = Vec::with_capacity(last_content - first_content + 1);
    for line in &lines[first_content..=last_content] {
        let content = line.trim();
        if content.is_empty() {
            formatted.push(String::new());
            continue;
        }

        let braces = inspect_braces(content);
        let line_depth = depth.saturating_sub(braces.leading_closing);
        formatted.push(format!("{}{}", "  ".repeat(line_depth), content));
        depth = depth
            .saturating_add(braces.opened)
            .saturating_sub(braces.closed);
    }

    format!("{}\n", formatted.join("\n"))
}

#[derive(Default)]
struct BraceCounts {
    opened: usize,
    closed: usize,
    leading_closing: usize,
}

fn inspect_braces(line: &str) -> BraceCounts {
    let mut counts = BraceCounts::default();
    let mut characters = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut before_first_token = true;

    while let Some(character) = characters.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '/' && characters.peek() == Some(&'/') {
            break;
        }

        match character {
            '"' => {
                in_string = true;
                before_first_token = false;
            }
            '{' => {
                counts.opened += 1;
                before_first_token = false;
            }
            '}' => {
                counts.closed += 1;
                if before_first_token {
                    counts.leading_closing += 1;
                }
            }
            character if character.is_whitespace() => {}
            _ => before_first_token = false,
        }
    }

    counts
}
