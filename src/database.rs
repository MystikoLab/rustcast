use std::{
    io::Cursor,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use arboard::ImageData;
use image::ImageReader;
use rusqlite::{Connection, params};

use crate::clipboard::ClipBoardContentType;

pub fn initialise_database() -> Connection {
    let conn =
        Connection::open("clipboard.db").expect("Couldn't open a connection to 'clipboard.db'");

    if !conn
        .table_exists(None, "clipboard_entries")
        .unwrap_or(false)
    {
        conn.execute_batch(include_str!("../migrations/db_init.sql"))
            .unwrap();
    };

    conn
}

pub fn load_clipboard(conn: &Connection) -> Vec<ClipBoardContentType> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT content_type, text_content, blob_content
         FROM clipboard_entries
         ORDER BY created_at DESC
         LIMIT 300",
    ) else {
        return vec![];
    };

    stmt.query_map([], |row| {
        let content_type: String = row.get(0)?;
        match content_type.as_str() {
            "text" => Ok(ClipBoardContentType::Text(row.get(1)?)),
            "url" => Ok(ClipBoardContentType::Url(row.get(1)?)),
            "image" => {
                let img_data: Vec<u8> = row.get(2)?;
                let image = ImageReader::new(Cursor::new(img_data))
                    .with_guessed_format()
                    .ok()
                    .and_then(|x| x.decode().ok())
                    .ok_or({
                        rusqlite::Error::InvalidColumnType(
                            0,
                            content_type,
                            rusqlite::types::Type::Blob,
                        )
                    })?;
                Ok(ClipBoardContentType::Image(ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: image.into_bytes().into(),
                }))
            }
            _ => Err(rusqlite::Error::InvalidColumnType(
                0,
                content_type,
                rusqlite::types::Type::Text,
            )),
        }
    })
    .map(|x| x.filter_map(|x| x.ok()).collect())
    .unwrap_or_default()
}

pub fn store_clipboard_content(conn: &Connection, content: &ClipBoardContentType) {
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
            "INSERT INTO clipboard_entries (content_type, blob_content, created_at, size_bytes)
             VALUES ('image', ?1, ?2, ?3)",
            params![bytes.bytes.to_vec(), now, bytes.bytes.len() as u32],
        ),
    }
    .unwrap();
}
