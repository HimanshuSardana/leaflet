use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Container formats leaflet reads as-is. Sniffed from the bytes, never from
/// the filename: a `.pdf` that is really an epub must not fool the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BookFormat {
    Pdf,
    Epub,
    Cbz,
}

impl BookFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Epub => "epub",
            Self::Cbz => "cbz",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "pdf" => Some(Self::Pdf),
            "epub" => Some(Self::Epub),
            "cbz" => Some(Self::Cbz),
            _ => None,
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Epub => "application/epub+zip",
            Self::Cbz => "application/vnd.comicbook+zip",
        }
    }

    pub fn extension(self) -> &'static str {
        self.as_str()
    }
}

/// A book as the API exposes it: metadata plus the blob's address, never the
/// blob itself.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub author: Option<String>,
    pub series: Option<String>,
    pub volume: Option<f64>,
    pub language: Option<String>,
    pub format: BookFormat,
    pub page_count: Option<i64>,
    pub byte_size: i64,
    /// sha256 of the stored bytes; stable across re-uploads.
    pub sha256: String,
    pub added_at: DateTime<Utc>,
}

/// Row shape straight from `books`.
#[derive(Debug, FromRow)]
pub struct BookRecord {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub series: Option<String>,
    pub volume: Option<f64>,
    pub language: Option<String>,
    pub format: String,
    pub page_count: Option<i64>,
    pub byte_size: i64,
    pub sha256: String,
    pub added_at: String,
}

impl BookRecord {
    pub fn into_book(self) -> Option<Book> {
        Some(Book {
            id: Uuid::parse_str(&self.id).ok()?,
            title: self.title,
            author: self.author,
            series: self.series,
            volume: self.volume,
            language: self.language,
            format: BookFormat::parse(&self.format)?,
            page_count: self.page_count,
            byte_size: self.byte_size,
            sha256: self.sha256,
            added_at: DateTime::parse_from_rfc3339(&self.added_at)
                .ok()?
                .with_timezone(&Utc),
        })
    }
}

/// What a filename says the book is called, when metadata extraction has not
/// run (or the file carries none): `old_man's_war.epub` -> `old man's war`.
pub fn title_from_filename(filename: &str) -> String {
    let stem = filename
        .rsplit('/')
        .next()
        .unwrap_or(filename)
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(filename);

    let cleaned: String = stem
        .chars()
        .map(|c| if c == '_' { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if cleaned.is_empty() {
        "Untitled".into()
    } else {
        cleaned.chars().take(200).collect()
    }
}

/// Detects the container from its first bytes. PDFs start with `%PDF-`; an
/// EPUB is a zip whose first entry stores the `application/epub+zip` mimetype,
/// which the spec requires to appear uncompressed at the head of the archive.
pub fn sniff_format(head: &[u8]) -> Option<BookFormat> {
    if head.starts_with(b"%PDF-") {
        return Some(BookFormat::Pdf);
    }
    if head.starts_with(b"PK\x03\x04") {
        return head
            .windows(20)
            .any(|window| window == b"application/epub+zip")
            .then_some(BookFormat::Epub);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_pdf_and_epub_and_rejects_the_rest() {
        assert_eq!(sniff_format(b"%PDF-1.7\n..."), Some(BookFormat::Pdf));
        let epub = b"PK\x03\x04\x14\x00\x00\x00\x08\x00\x1b\x00mimetypeapplication/epub+zip";
        assert_eq!(sniff_format(epub), Some(BookFormat::Epub));
        assert_eq!(sniff_format(b"PK\x03\x04not-an-epub-zip"), None);
        assert_eq!(sniff_format(b"hello world"), None);
        assert_eq!(sniff_format(b""), None);
    }

    #[test]
    fn titles_come_from_names_without_lying_about_them() {
        assert_eq!(title_from_filename("old_man's_war.epub"), "old man's war");
        assert_eq!(title_from_filename("/books/dune.PDF"), "dune");
        assert_eq!(title_from_filename("  spaced   out .pdf"), "spaced out");
        assert_eq!(title_from_filename(".pdf"), "Untitled");
        assert_eq!(title_from_filename("no-extension"), "no-extension");
        assert_eq!(title_from_filename(&"x".repeat(400)).len(), 200);
    }

    #[test]
    fn formats_round_trip_through_the_database_encoding() {
        for format in [BookFormat::Pdf, BookFormat::Epub, BookFormat::Cbz] {
            assert_eq!(BookFormat::parse(format.as_str()), Some(format));
        }
        assert_eq!(BookFormat::parse("mobi"), None);
    }
}
