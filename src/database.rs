use std::{
    borrow::Cow,
    env,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use arboard::ImageData;
use rusqlite::{Connection, params};

use crate::clipboard::ClipBoardContentType;

pub fn initialise_database() -> Connection {
    try_initialise_database().expect("Couldn't open a connection to 'clipboard.db'")
}

fn database_path() -> std::path::PathBuf {
    let current_exe = env::current_exe()
        .ok()
        .and_then(|x| x.parent().map(|x| x.to_path_buf()))
        .unwrap_or(Path::new("/tmp").to_path_buf());

    current_exe.join(Path::new("clipboard.db"))
}

fn try_initialise_database() -> rusqlite::Result<Connection> {
    initialise_database_at(&database_path())
}

fn initialise_database_at(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;

    let table_exists = conn
        .table_exists(None, "clipboard_entries")
        .unwrap_or(false);

    if !table_exists {
        conn.execute_batch(include_str!("../migrations/db_init.sql"))?;
        conn.pragma_update(None, "user_version", 1)?;
    } else {
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version < 1 {
            for column in ["image_width", "image_height"] {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(
                    SELECT 1 FROM pragma_table_info('clipboard_entries') WHERE name = ?1
                )",
                    [column],
                    |row| row.get(0),
                )?;

                if !exists {
                    conn.execute(
                        &format!("ALTER TABLE clipboard_entries ADD COLUMN {column} INTEGER"),
                        [],
                    )?;
                }
            }

            conn.execute_batch(
                "DELETE FROM clipboard_entries
                 WHERE (content_type = 'image' AND (
                     blob_content IS NULL OR image_width IS NULL OR image_height IS NULL
                     OR length(blob_content) != image_width * image_height * 4
                 )) OR (content_type IN ('text', 'url') AND text_content IS NULL);
                 DELETE FROM clipboard_entries
                 WHERE id NOT IN (
                     SELECT id FROM clipboard_entries
                     ORDER BY created_at DESC, id DESC LIMIT 300
                 );
                 PRAGMA user_version = 1;",
            )?;
        }
    }

    Ok(conn)
}

pub async fn load_clipboard_in_background() -> Result<Vec<ClipBoardContentType>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = try_initialise_database().map_err(|error| error.to_string())?;
        try_load_clipboard(&conn).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
pub fn load_clipboard(conn: &Connection) -> Vec<ClipBoardContentType> {
    try_load_clipboard(conn).unwrap_or_default()
}

fn try_load_clipboard(conn: &Connection) -> rusqlite::Result<Vec<ClipBoardContentType>> {
    let mut stmt = conn.prepare(
        "SELECT content_type, text_content, blob_content, image_width, image_height
         FROM clipboard_entries
         ORDER BY created_at DESC, id DESC
         LIMIT 300",
    )?;

    let rows = stmt.query_map([], |row| {
        let content_type: String = row.get(0)?;
        match content_type.as_str() {
            "text" => Ok(ClipBoardContentType::Text(row.get(1)?)),
            "url" => Ok(ClipBoardContentType::Url(row.get(1)?)),
            "image" => {
                let bytes: Vec<u8> = row.get(2)?;
                let width = row.get::<_, u32>(3)? as usize;
                let height = row.get::<_, u32>(4)? as usize;

                if bytes.len() != width.saturating_mul(height).saturating_mul(4) {
                    return Err(rusqlite::Error::InvalidColumnType(
                        2,
                        content_type,
                        rusqlite::types::Type::Blob,
                    ));
                }

                Ok(ClipBoardContentType::Image(ImageData {
                    width,
                    height,
                    bytes: Cow::Owned(bytes),
                }))
            }
            _ => Err(rusqlite::Error::InvalidColumnType(
                0,
                content_type,
                rusqlite::types::Type::Text,
            )),
        }
    })?;

    rows.collect()
}

pub fn store_clipboard_content(
    conn: &Connection,
    content: &ClipBoardContentType,
) -> rusqlite::Result<()> {
    let transaction = conn.unchecked_transaction()?;
    delete_clipboard_content(&transaction, content)?;
    insert_clipboard_content(&transaction, content)?;
    transaction.commit()
}

fn insert_clipboard_content(
    conn: &Connection,
    content: &ClipBoardContentType,
) -> rusqlite::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_millis(1))
        .as_millis() as i64;

    match content {
        ClipBoardContentType::Text(s) => conn.execute(
            "INSERT INTO clipboard_entries (content_type, text_content, created_at, size_bytes)
             VALUES ('text', ?1, ?2, ?3)",
            params![s, now, s.len() as u32],
        ),
        ClipBoardContentType::Url(s) => conn.execute(
            "INSERT INTO clipboard_entries (content_type, text_content, created_at, size_bytes)
             VALUES ('url', ?1, ?2, ?3)",
            params![s, now, s.len() as u32],
        ),
        ClipBoardContentType::Image(bytes) => conn.execute(
            "INSERT INTO clipboard_entries (
                content_type, blob_content, image_width, image_height, created_at, size_bytes
             ) VALUES ('image', ?1, ?2, ?3, ?4, ?5)",
            params![
                bytes.bytes.as_ref(),
                bytes.width as i64,
                bytes.height as i64,
                now,
                bytes.bytes.len() as i64
            ],
        ),
    }?;

    conn.execute(
        "DELETE FROM clipboard_entries
         WHERE id NOT IN (
             SELECT id FROM clipboard_entries ORDER BY created_at DESC, id DESC LIMIT 300
         )",
        [],
    )?;
    Ok(())
}

pub fn delete_clipboard_content(
    conn: &Connection,
    content: &ClipBoardContentType,
) -> rusqlite::Result<()> {
    match content {
        ClipBoardContentType::Text(text) => conn.execute(
            "DELETE FROM clipboard_entries WHERE content_type = 'text' AND text_content = ?1",
            [text],
        ),
        ClipBoardContentType::Url(url) => conn.execute(
            "DELETE FROM clipboard_entries WHERE content_type = 'url' AND text_content = ?1",
            [url],
        ),
        ClipBoardContentType::Image(image) => conn.execute(
            "DELETE FROM clipboard_entries
             WHERE content_type = 'image' AND blob_content = ?1
               AND image_width = ?2 AND image_height = ?3",
            params![
                image.bytes.as_ref(),
                image.width as i64,
                image.height as i64
            ],
        ),
    }
    .map(|_| ())
}

pub fn update_clipboard_content(
    conn: &Connection,
    old: &ClipBoardContentType,
    new: &ClipBoardContentType,
) -> rusqlite::Result<()> {
    let transaction = conn.unchecked_transaction()?;
    delete_clipboard_content(&transaction, old)?;
    delete_clipboard_content(&transaction, new)?;
    insert_clipboard_content(&transaction, new)?;
    transaction.commit()
}

pub fn clear_clipboard(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM clipboard_entries", [])
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_database() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = initialise_database_at(&dir.path().join("clipboard.db")).unwrap();
        (dir, conn)
    }

    #[test]
    fn image_round_trips_without_decoding() {
        let (_dir, conn) = test_database();
        let image = ClipBoardContentType::Image(ImageData {
            width: 1,
            height: 1,
            bytes: Cow::Owned(vec![1, 2, 3, 4]),
        });

        store_clipboard_content(&conn, &image).unwrap();

        assert_eq!(load_clipboard(&conn), vec![image]);
    }

    #[test]
    fn stored_history_is_limited_to_three_hundred_entries() {
        let (_dir, conn) = test_database();

        for index in 0..301 {
            store_clipboard_content(&conn, &ClipBoardContentType::Text(format!("entry {index}")))
                .unwrap();
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 300);
        assert!(
            !load_clipboard(&conn).contains(&ClipBoardContentType::Text("entry 0".to_string()))
        );
    }

    #[test]
    fn delete_and_clear_are_persisted() {
        let (_dir, conn) = test_database();
        let first = ClipBoardContentType::Text("first".to_string());
        let second = ClipBoardContentType::Url("https://rustcast.app".to_string());
        store_clipboard_content(&conn, &first).unwrap();
        store_clipboard_content(&conn, &second).unwrap();

        delete_clipboard_content(&conn, &first).unwrap();
        assert_eq!(load_clipboard(&conn), vec![second]);

        clear_clipboard(&conn).unwrap();
        assert!(load_clipboard(&conn).is_empty());
    }

    #[test]
    fn existing_database_gets_image_dimensions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("clipboard.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE clipboard_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL,
                text_content TEXT,
                blob_content BLOB,
                created_at INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL
            );
            INSERT INTO clipboard_entries (
                content_type, text_content, created_at, size_bytes
            ) VALUES ('text', 'keep me', 1000, 7);
            INSERT INTO clipboard_entries (
                content_type, blob_content, created_at, size_bytes
            ) VALUES ('image', X'01020304', 2, 4);",
        )
        .unwrap();
        for index in 0..301 {
            conn.execute(
                "INSERT INTO clipboard_entries (
                    content_type, text_content, created_at, size_bytes
                 ) VALUES ('text', ?1, ?2, 4)",
                params![format!("entry {index}"), index],
            )
            .unwrap();
        }
        drop(conn);

        let conn = initialise_database_at(&path).unwrap();
        let columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('clipboard_entries')
                 WHERE name IN ('image_width', 'image_height')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(columns, 2);
        let content = load_clipboard(&conn);
        assert_eq!(content.len(), 300);
        assert!(content.contains(&ClipBoardContentType::Text("keep me".to_string())));
        assert!(
            !content
                .iter()
                .any(|item| matches!(item, ClipBoardContentType::Image(_)))
        );
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn update_is_persisted() {
        let (_dir, conn) = test_database();
        let old = ClipBoardContentType::Text("old".to_string());
        let new = ClipBoardContentType::Text("new".to_string());
        store_clipboard_content(&conn, &old).unwrap();
        store_clipboard_content(&conn, &new).unwrap();

        update_clipboard_content(&conn, &old, &new).unwrap();

        assert_eq!(load_clipboard(&conn), vec![new]);
    }
}
