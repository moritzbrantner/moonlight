use serde_json::Value;

const REDACTED: &str = "[redacted]";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Root,
    Key(String),
    Index(usize),
    AnyKey,
    AnyIndex,
}

pub(super) fn matches_path(path: &str, pattern: &str) -> bool {
    let Some(path_segments) = parse(path, false) else {
        return false;
    };
    let Some(pattern_segments) = parse(pattern, true) else {
        return false;
    };
    if path_segments.len() != pattern_segments.len() {
        return false;
    }

    path_segments
        .iter()
        .zip(pattern_segments.iter())
        .all(|(path, pattern)| match pattern {
            Segment::AnyKey => matches!(path, Segment::Key(_)),
            Segment::AnyIndex => matches!(path, Segment::Index(_)),
            _ => path == pattern,
        })
}

pub(super) fn matches_any_path(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| matches_path(path, pattern))
}

pub(super) fn redact_value_at_path(value: &mut Value, path: &str) -> bool {
    let Some(segments) = parse(path, false) else {
        return false;
    };
    redact_matches(value, &segments[1..])
}

pub(super) fn redact_value_at_matching_paths(value: &mut Value, pattern: &str) -> bool {
    let Some(pattern) = parse(pattern, true) else {
        return false;
    };
    redact_matches(value, &pattern[1..])
}

fn redact_matches(value: &mut Value, segments: &[Segment]) -> bool {
    let Some((segment, remaining)) = segments.split_first() else {
        *value = Value::String(REDACTED.to_string());
        return true;
    };

    match segment {
        Segment::Key(key) => value
            .get_mut(key)
            .is_some_and(|next| redact_matches(next, remaining)),
        Segment::Index(index) => value
            .get_mut(*index)
            .is_some_and(|next| redact_matches(next, remaining)),
        Segment::AnyKey => {
            let Some(map) = value.as_object_mut() else {
                return false;
            };
            let mut changed = false;
            for next in map.values_mut() {
                changed = redact_matches(next, remaining) || changed;
            }
            changed
        }
        Segment::AnyIndex => {
            let Some(items) = value.as_array_mut() else {
                return false;
            };
            let mut changed = false;
            for next in items {
                changed = redact_matches(next, remaining) || changed;
            }
            changed
        }
        Segment::Root => false,
    }
}

fn parse(path: &str, allow_wildcards: bool) -> Option<Vec<Segment>> {
    let mut chars = path.chars().peekable();
    if chars.next()? != '$' {
        return None;
    }

    let mut segments = vec![Segment::Root];
    while let Some(next) = chars.peek().copied() {
        match next {
            '.' => {
                chars.next();
                let mut key = String::new();
                while let Some(ch) = chars.peek().copied() {
                    if ch == '.' || ch == '[' {
                        break;
                    }
                    key.push(ch);
                    chars.next();
                }
                if key.is_empty() {
                    return None;
                }
                if allow_wildcards && key == "*" {
                    segments.push(Segment::AnyKey);
                } else {
                    segments.push(Segment::Key(key));
                }
            }
            '[' => {
                chars.next();
                let mut index = String::new();
                while let Some(ch) = chars.peek().copied() {
                    if ch == ']' {
                        break;
                    }
                    index.push(ch);
                    chars.next();
                }
                if chars.next() != Some(']') {
                    return None;
                }
                if allow_wildcards && index == "*" {
                    segments.push(Segment::AnyIndex);
                } else {
                    segments.push(Segment::Index(index.parse().ok()?));
                }
            }
            _ => return None,
        }
    }

    Some(segments)
}
