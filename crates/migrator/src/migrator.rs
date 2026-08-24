//! ## When to create a migration and why?
//! A migration is necessary when keymap actions or settings are renamed or transformed (e.g., from an array to a string, a string to an array, a boolean to an enum, etc.).
//!
//! This ensures that users with outdated settings are automatically updated to use the corresponding new settings internally.
//! It also provides a quick way to migrate their existing settings to the latest state using button in UI.
//!
//! ## How to create a migration?
//! Migrations use Tree-sitter to query commonly used patterns, such as actions with a string or actions with an array where the second argument is an object, etc.
//! Once queried, *you can filter out the modified items* and write the replacement logic.
//!
//! You *must not* modify previous migrations; always create new ones instead.
//! This is important because if a user is in an intermediate state, they can smoothly transition to the latest state.
//! Modifying existing migrations means they will only work for users upgrading from version x-1 to x, but not from x-2 to x, and so on, where x is the latest version.
//!
//! You only need to write replacement logic for x-1 to x because you can be certain that, internally, every user will be at x-1, regardless of their on disk state.

use anyhow::{Context as _, Result};
use settings_json::{infer_json_indent_size, parse_json_with_comments, update_value_in_json_text};
use std::{cmp::Reverse, ops::Range};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Query, QueryMatch};

mod migrations;

fn migrate(text: &str, patterns: MigrationPatterns, query: &Query) -> Result<Option<String>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_json::LANGUAGE.into())?;
    let syntax_tree = parser.parse(text, None).context("failed to parse settings")?;

    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(query, syntax_tree.root_node(), text.as_bytes());

    let mut edits = vec![];
    while let Some(mat) = matches.next() {
        if let Some((_, callback)) = patterns.get(mat.pattern_index) {
            edits.extend(callback(text, mat, query));
        }
    }

    edits.sort_by_key(|(range, _)| (range.start, Reverse(range.end)));
    edits.dedup_by(|(range_b, _), (range_a, _)| range_a.contains(&range_b.start) || range_a.contains(&range_b.end));

    if edits.is_empty() {
        Ok(None)
    } else {
        let mut new_text = text.to_string();
        for (range, replacement) in edits.iter().rev() {
            new_text.replace_range(range.clone(), replacement);
        }
        if new_text == text {
            log::error!(
                "Edits computed for configuration migration do not cause a change: {:?}",
                edits
            );
            Ok(None)
        } else {
            Ok(Some(new_text))
        }
    }
}

/// Runs the provided migrations on the given text.
/// Will automatically return `Ok(None)` if there's no content to migrate.
fn run_migrations(text: &str, migrations: &[MigrationType]) -> Result<Option<String>> {
    if text.is_empty() {
        return Ok(None);
    }

    let mut current_text = text.to_string();
    let mut result: Option<String> = None;
    let json_indent_size = infer_json_indent_size(&current_text);
    for migration in migrations.iter() {
        let migrated_text = match migration {
            MigrationType::TreeSitter(patterns, query) => migrate(&current_text, patterns, query)?,
            MigrationType::Json(callback) => {
                if current_text.trim().is_empty() {
                    return Ok(None);
                }
                let old_content: serde_json_lenient::Value = parse_json_with_comments(&current_text)?;
                let old_value = serde_json::to_value(&old_content).unwrap();
                let mut new_value = old_value.clone();
                callback(&mut new_value)?;
                if new_value != old_value {
                    let mut current = current_text.clone();
                    let mut edits = vec![];
                    update_value_in_json_text(
                        &mut current,
                        &mut vec![],
                        json_indent_size,
                        &old_value,
                        &new_value,
                        &mut edits,
                    );
                    let mut migrated_text = current_text.clone();
                    for (range, replacement) in edits.into_iter() {
                        migrated_text.replace_range(range, &replacement);
                    }
                    Some(migrated_text)
                } else {
                    None
                }
            }
        };
        if let Some(migrated_text) = migrated_text {
            current_text = migrated_text.clone();
            result = Some(migrated_text);
        }
    }
    Ok(result.filter(|new_text| text != new_text))
}

pub fn migrate_keymap(text: &str) -> Result<Option<String>> {
    let migrations: &[MigrationType] = &[];
    run_migrations(text, migrations)
}

#[allow(dead_code)]
enum MigrationType<'a> {
    TreeSitter(MigrationPatterns, &'a Query),
    Json(fn(&mut serde_json::Value) -> Result<()>),
}

pub fn migrate_settings(text: &str) -> Result<Option<String>> {
    let migrations: &[MigrationType] = &[
        MigrationType::Json(migrations::m_0001::removed_settings),
        MigrationType::Json(migrations::m_0001::rename_status_bar_show),
    ];
    run_migrations(text, migrations)
}

pub type MigrationPatterns = &'static [(
    &'static str,
    fn(&str, &QueryMatch, &Query) -> Option<(Range<usize>, String)>,
)];

#[allow(unused_macros)]
macro_rules! define_query {
    ($var_name:ident, $patterns_path:path) => {
        static $var_name: LazyLock<Query> = LazyLock::new(|| {
            Query::new(
                &tree_sitter_json::LANGUAGE.into(),
                &$patterns_path.iter().map(|pattern| pattern.0).collect::<String>(),
            )
            .unwrap()
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_migrated_correctly(migrated: Option<String>, expected: Option<&str>) {
        match (&migrated, &expected) {
            (Some(migrated), Some(expected)) => {
                pretty_assertions::assert_str_eq!(expected, migrated);
            }
            _ => {
                pretty_assertions::assert_eq!(migrated.as_deref(), expected);
            }
        }
    }

    #[allow(dead_code)]
    #[track_caller]
    fn assert_migrate_settings_with_migrations(migrations: &[MigrationType], input: &str, output: Option<&str>) {
        let migrated = run_migrations(input, migrations).unwrap();
        assert_migrated_correctly(migrated.clone(), output);

        // expect that rerunning the migration does not result in another migration
        if let Some(migrated) = migrated {
            let rerun = run_migrations(&migrated, migrations).unwrap();
            assert_migrated_correctly(rerun, None);
        }
    }
}
