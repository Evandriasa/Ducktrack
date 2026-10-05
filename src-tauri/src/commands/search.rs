use rusqlite::{params, Row};
use tauri::State;

use super::super::database::AppState;
use super::super::error::AppError;
use super::super::models::{SearchItem, SearchResult};

fn escape_fts_token(token: &str) -> String {
    token.chars().filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'').collect()
}

fn escape_like(value: &str) -> String {
    value.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

#[tauri::command(rename_all = "snake_case")]
pub fn global_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let tokens: Vec<String> = query
        .split_whitespace()
        .map(escape_fts_token)
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let match_expr = tokens
        .iter()
        .map(|t| format!("\"{t}\"*"))
        .collect::<Vec<_>>()
        .join(" ");

    let results = (|| -> rusqlite::Result<Vec<(String, String, String, i64, String)>> {
        let mut stmt = db.prepare(
            "SELECT si.category, si.title, si.body, si.item_id, si.created_at
             FROM fts_workspace f
             JOIN search_index si ON si.id = f.rowid
             WHERE fts_workspace MATCH ?1 AND si.user_id = ?2
             ORDER BY si.created_at DESC
             LIMIT 200",
        )?;
        let rows = stmt.query_map(params![match_expr, uid], |row: &Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        rows.collect()
    })();

    let rows = match results {
        Ok(rows) => rows,
        Err(_) => {
            // FTS syntax/semantics error or no match: fall back to LIKE scan.
            let mut stmt = db.prepare(
                "SELECT category, title, body, item_id, created_at
                 FROM search_index si
                 WHERE (si.title LIKE ?1 ESCAPE '\\' OR si.body LIKE ?1 ESCAPE '\\') AND si.user_id = ?2
                 ORDER BY si.created_at DESC
                 LIMIT 200",
            )?;
            let pattern = format!("%{}%", escape_like(&query));
            let rows = stmt.query_map(params![pattern, uid], |row: &Row<'_>| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
    };

    let mut grouped: Vec<SearchResult> = Vec::new();
    let mut group_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (category, title, body, item_id, created_at) in rows {
        let item = SearchItem {
            title,
            body,
            ref_id: item_id,
            created_at,
        };
        match group_index.get(&category).copied() {
            Some(ix) => grouped[ix].items.push(item),
            None => {
                group_index.insert(category.clone(), grouped.len());
                grouped.push(SearchResult {
                    category,
                    items: vec![item],
                });
            }
        }
    }

    Ok(grouped)
}