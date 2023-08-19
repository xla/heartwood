use std::path::PathBuf;

use radicle::cob::patch::{CodeLocation, CodeRange};
use radicle_surf::diff::{Added, Deleted, FileDiff, Hunk, Modification, Modified};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comment(pub String, pub CodeLocation);

#[derive(Debug)]
enum Line {
    Diff,
    Newline,
    Comment,
}

impl<'a> From<&'a str> for Line {
    fn from(s: &'a str) -> Self {
        if s.is_empty() {
            return Line::Newline;
        }

        match s.chars().peekable().peek().unwrap() {
            '+' | '-' | ' ' => Line::Diff,
            '\n' => Line::Newline,
            _ => Line::Comment,
        }
    }
}

/// Parses multi-line comments from a user annotated git diff hunk. Every consecutive line not
/// starting with a `+`, `-` or a single ` ` is considered part of a single comment. Comments
/// support common markdown syntax, including empty lines. The CodeLocation of a comment is the
/// range of lines above the start of the comment up to the either the next comment, a single empty line, or the start of the hunk.
pub fn parse_hunk_comments(
    file: &FileDiff,
    hunk: &Hunk<Modification>,
    input: String,
) -> anyhow::Result<Vec<Comment>> {
    // dbg!(&file, &hunk, &input);

    let mut comments: Vec<Comment> = Vec::new();
    let mut body: Option<String> = None;
    let location = CodeLocation {
        path: file_path(file),
        old: None,
        new: None,
    };
    let lines = input.lines().peekable();
    let mut offset = 0;

    for line in lines {
        dbg!(&line, Line::from(line), &body);
        match Line::from(line) {
            Line::Diff => {
                offset += 1;

                if let Some(b) = body {
                    body = None;

                    if !b.is_empty() && !b.chars().all(|c| c.is_whitespace() || c == '\n') {
                        comments.push(Comment(b, location.clone()));
                    }
                }
            }
            Line::Newline => {
                if let Some(mut b) = body {
                    b.push('\n');
                    b.push_str(line);
                    body = Some(b);
                }
                // TODO(xla): Move code location if not commenting.
            }
            Line::Comment => {
                if let Some(mut b) = body {
                    b.push('\n');
                    b.push_str(line);
                    body = Some(b);
                } else {
                    body = Some(line.to_owned())
                }
            }
        }
    }

    Ok(comments)
}

fn file_path(file: &FileDiff) -> PathBuf {
    match file {
        FileDiff::Added(Added { path, .. }) => path.clone(),
        FileDiff::Copied(_) => todo!(),
        FileDiff::Deleted(Deleted { path, .. }) => path.clone(),
        FileDiff::Modified(Modified { path, .. }) => path.clone(),
        FileDiff::Moved(_) => todo!(),
    }
}

#[cfg(test)]
mod test {
    use radicle::cob::patch::{CodeLocation, CodeRange};
    use radicle::git::raw as git2;
    use radicle_surf::diff::{Diff, DiffContent, FileDiff, Modified};

    use super::{parse_hunk_comments, Comment};

    #[test]
    fn test_parse_hunk_comments() {
        let buf = r#"
diff --git a/MENU.txt b/MENU.txt
index 4e2e828..3af9741 100644
--- a/MENU.txt
+++ b/MENU.txt
@@ -12,4 +12,5 @@ Fried Shrimp Basket
 
 Sides
 -----
-French Fries
+French Fries!
+Garlic Green Beans
"#;

        let diff = git2::Diff::from_buffer(buf.as_bytes()).unwrap();
        let diff = Diff::try_from(diff).unwrap();
        let file = diff.files().next().unwrap();
        let hunk = match file {
            FileDiff::Modified(Modified { diff, .. }) => match diff {
                DiffContent::Plain { hunks, .. } => hunks.0[0].clone(),
                _ => panic!("expected hunks"),
            },
            _ => panic!("expected modified file"),
        };

        let input = r#" 
 Sides
 -----

-French Fries

First comment.

+French Fries!
+Garlic Green Beans

Second comment.

Which is multiline.
        "#;

        assert_eq!(
            parse_hunk_comments(&file, &hunk, input.to_owned()).unwrap(),
            vec![
                Comment(
                    "First comment.\n".to_owned(),
                    CodeLocation {
                        path: "MENU.txt".into(),
                        old: None,
                        new: None,
                    }
                ),
                Comment(
                    "Second comment.\n\nWhich is multiline.".to_owned(),
                    CodeLocation {
                        path: "MENU.txt".into(),
                        old: None,
                        new: None,
                    }
                ),
            ],
        )
    }
}
