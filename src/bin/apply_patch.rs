use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
enum FileOp {
    Add { path: String, lines: Vec<String> },
    Delete { path: String },
    Update {
        path: String,
        move_to: Option<String>,
        hunks: Vec<Hunk>,
    },
}

#[derive(Debug, Clone)]
struct Hunk {
    lines: Vec<HunkLine>,
}

#[derive(Debug, Clone)]
enum HunkLine {
    Context(String),
    Remove(String),
    Add(String),
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let patch_text = if args.is_empty() {
        use std::io::Read;
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("failed to read patch from stdin")?;
        input
    } else {
        args.join(" ")
    };

    if patch_text.trim().is_empty() {
        bail!("empty patch input")
    }

    let ops = parse_patch(&patch_text)?;
    apply_ops(&ops)?;
    Ok(())
}

fn parse_patch(input: &str) -> Result<Vec<FileOp>> {
    let lines: Vec<&str> = input
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();

    if lines.is_empty() {
        bail!("patch is empty")
    }

    let mut index = 0;
    require_line(&lines, index, "*** Begin Patch")?;
    index += 1;

    let mut ops = Vec::new();

    while index < lines.len() {
        let line = lines[index];
        if line == "*** End Patch" {
            index += 1;
            if index != lines.len() {
                bail!("unexpected content after *** End Patch")
            }
            return Ok(ops);
        }

        if let Some(path) = line.strip_prefix("*** Add File: ") {
            validate_relative_path(path)?;
            index += 1;
            let mut added = Vec::new();
            while index < lines.len() {
                let current = lines[index];
                if is_file_header(current) || current == "*** End Patch" {
                    break;
                }
                let content = current
                    .strip_prefix('+')
                    .ok_or_else(|| anyhow!("invalid add-file line: expected '+' prefix"))?;
                added.push(content.to_string());
                index += 1;
            }
            ops.push(FileOp::Add {
                path: path.to_string(),
                lines: added,
            });
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Delete File: ") {
            validate_relative_path(path)?;
            ops.push(FileOp::Delete {
                path: path.to_string(),
            });
            index += 1;
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Update File: ") {
            validate_relative_path(path)?;
            index += 1;

            let mut move_to = None;
            if index < lines.len() {
                if let Some(new_path) = lines[index].strip_prefix("*** Move to: ") {
                    validate_relative_path(new_path)?;
                    move_to = Some(new_path.to_string());
                    index += 1;
                }
            }

            let mut hunks = Vec::new();
            while index < lines.len() {
                let current = lines[index];
                if is_file_header(current) || current == "*** End Patch" {
                    break;
                }
                if !current.starts_with("@@") {
                    bail!("expected hunk header '@@', got: {}", current);
                }

                index += 1;
                let mut hunk_lines = Vec::new();
                while index < lines.len() {
                    let hline = lines[index];
                    if hline.starts_with("@@") || is_file_header(hline) || hline == "*** End Patch" {
                        break;
                    }
                    if hline == "*** End of File" {
                        index += 1;
                        break;
                    }
                    let mut chars = hline.chars();
                    let marker = chars
                        .next()
                        .ok_or_else(|| anyhow!("empty hunk line is invalid"))?;
                    let tail: String = chars.collect();
                    match marker {
                        ' ' => hunk_lines.push(HunkLine::Context(tail)),
                        '-' => hunk_lines.push(HunkLine::Remove(tail)),
                        '+' => hunk_lines.push(HunkLine::Add(tail)),
                        _ => bail!("invalid hunk line prefix '{}'", marker),
                    }
                    index += 1;
                }

                if hunk_lines.is_empty() {
                    bail!("empty hunk is invalid")
                }
                hunks.push(Hunk { lines: hunk_lines });
            }

            if hunks.is_empty() {
                bail!("update operation for '{}' has no hunks", path)
            }

            ops.push(FileOp::Update {
                path: path.to_string(),
                move_to,
                hunks,
            });
            continue;
        }

        bail!("unknown patch section header: {}", line)
    }

    bail!("missing *** End Patch")
}

fn require_line(lines: &[&str], index: usize, expected: &str) -> Result<()> {
    let found = lines
        .get(index)
        .copied()
        .ok_or_else(|| anyhow!("patch ended early; expected {}", expected))?;
    if found != expected {
        bail!("expected '{}', got '{}'", expected, found);
    }
    Ok(())
}

fn is_file_header(line: &str) -> bool {
    line.starts_with("*** Add File: ")
        || line.starts_with("*** Delete File: ")
        || line.starts_with("*** Update File: ")
}

fn validate_relative_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        bail!("path cannot be empty")
    }

    let parsed = Path::new(path);
    if parsed.is_absolute() {
        bail!("path must be relative: {}", path)
    }

    for component in parsed.components() {
        match component {
            Component::CurDir | Component::Normal(_) => {}
            Component::ParentDir => bail!("parent path '..' is not allowed: {}", path),
            _ => bail!("invalid path component in {}", path),
        }
    }

    Ok(())
}

fn apply_ops(ops: &[FileOp]) -> Result<()> {
    for op in ops {
        match op {
            FileOp::Add { path, lines } => apply_add(path, lines)?,
            FileOp::Delete { path } => apply_delete(path)?,
            FileOp::Update {
                path,
                move_to,
                hunks,
            } => apply_update(path, move_to.as_deref(), hunks)?,
        }
    }
    Ok(())
}

fn apply_add(path: &str, lines: &[String]) -> Result<()> {
    let full_path = PathBuf::from(path);
    if full_path.exists() {
        bail!("add file failed: '{}' already exists", path)
    }

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directories for {}", path))?;
    }

    fs::write(&full_path, normalize_lines(lines))
        .with_context(|| format!("failed to write {}", path))?;
    Ok(())
}

fn apply_delete(path: &str) -> Result<()> {
    let full_path = PathBuf::from(path);
    if !full_path.exists() {
        bail!("delete file failed: '{}' does not exist", path)
    }

    fs::remove_file(&full_path).with_context(|| format!("failed to delete {}", path))?;
    Ok(())
}

fn apply_update(path: &str, move_to: Option<&str>, hunks: &[Hunk]) -> Result<()> {
    let source_path = PathBuf::from(path);
    if !source_path.exists() {
        bail!("update file failed: '{}' does not exist", path)
    }

    let original = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read {}", path))?;
    let updated = apply_hunks(&original, hunks).with_context(|| format!("failed to patch {}", path))?;

    let dest_path = move_to.map(PathBuf::from).unwrap_or_else(|| source_path.clone());
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directories for {}", dest_path.display()))?;
    }

    fs::write(&dest_path, updated)
        .with_context(|| format!("failed to write {}", dest_path.display()))?;

    if let Some(target) = move_to {
        if target != path {
            fs::remove_file(&source_path)
                .with_context(|| format!("failed to remove moved source file {}", path))?;
        }
    }

    Ok(())
}

fn apply_hunks(original: &str, hunks: &[Hunk]) -> Result<String> {
    let mut lines: Vec<String> = original
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect();
    let mut cursor = 0usize;

    for hunk in hunks {
        let expected_old: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                HunkLine::Context(text) | HunkLine::Remove(text) => Some(text.as_str()),
                HunkLine::Add(_) => None,
            })
            .collect();

        let replacement: Vec<String> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                HunkLine::Context(text) | HunkLine::Add(text) => Some(text.clone()),
                HunkLine::Remove(_) => None,
            })
            .collect();

        let match_pos = find_match(&lines, &expected_old, cursor)
            .or_else(|| find_match(&lines, &expected_old, 0))
            .ok_or_else(|| anyhow!("could not locate hunk context in target file"))?;

        let old_len = expected_old.len();
        lines.splice(match_pos..(match_pos + old_len), replacement.clone());
        cursor = match_pos + replacement.len();
    }

    let mut output = lines.join("\n");
    if original.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn find_match(lines: &[String], expected_old: &[&str], start: usize) -> Option<usize> {
    if expected_old.is_empty() {
        return Some(start.min(lines.len()));
    }
    if expected_old.len() > lines.len() || start > lines.len() {
        return None;
    }

    let end = lines.len() - expected_old.len();
    for idx in start..=end {
        let window = &lines[idx..idx + expected_old.len()];
        if window
            .iter()
            .zip(expected_old.iter())
            .all(|(a, b)| a == b)
        {
            return Some(idx);
        }
    }
    None
}

fn normalize_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_add_update_delete() {
        let patch = "*** Begin Patch\n*** Add File: hello.txt\n+Hello\n*** Update File: src/a.rs\n@@\n-old\n+new\n*** Delete File: stale.txt\n*** End Patch\n";
        let ops = parse_patch(patch).expect("patch should parse");
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn reject_absolute_path() {
        let patch = "*** Begin Patch\n*** Add File: /tmp/evil\n+nope\n*** End Patch\n";
        let err = parse_patch(patch).expect_err("absolute path should fail");
        assert!(err.to_string().contains("path must be relative"));
    }

    #[test]
    fn apply_single_hunk() {
        let original = "a\nb\nc\n";
        let hunk = Hunk {
            lines: vec![
                HunkLine::Context("a".to_string()),
                HunkLine::Remove("b".to_string()),
                HunkLine::Add("x".to_string()),
                HunkLine::Context("c".to_string()),
            ],
        };

        let out = apply_hunks(original, &[hunk]).expect("hunk should apply");
        assert_eq!(out, "a\nx\nc\n");
    }
}
