//! CSV export with CSV-injection ("formula injection") defense (docs/11 S8).
//! Pure, no I/O. A cell whose text comes from a crawled page (anchor text,
//! error strings) could be `=cmd|…` or `@SUM(...)`; opened in Excel/Sheets that
//! executes. We neutralize any field that begins with a formula trigger by
//! prefixing a single quote, and quote fields per RFC 4180 so the output is both
//! safe and a valid CSV that re-imports.

use std::borrow::Cow;

/// Prefix a leading formula trigger (`= + - @`, tab, CR) with `'` so a
/// spreadsheet treats the cell as text, not a formula. Everything else is
/// returned unchanged.
pub fn sanitize_field(s: &str) -> Cow<'_, str> {
    let trigger = matches!(
        s.as_bytes().first(),
        Some(b'=' | b'+' | b'-' | b'@' | b'\t' | b'\r')
    );
    if trigger {
        let mut out = String::with_capacity(s.len() + 1);
        out.push('\'');
        out.push_str(s);
        Cow::Owned(out)
    } else {
        Cow::Borrowed(s)
    }
}

/// RFC 4180 field quoting, applied after the injection guard. A field is quoted
/// when it contains a comma, quote, CR or LF; embedded quotes are doubled.
fn encode_field(s: &str) -> String {
    let guarded = sanitize_field(s);
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded.into_owned()
    }
}

fn write_row<'a>(out: &mut String, fields: impl Iterator<Item = &'a str>) {
    let mut first = true;
    for f in fields {
        if !first {
            out.push(',');
        }
        out.push_str(&encode_field(f));
        first = false;
    }
    out.push_str("\r\n"); // RFC 4180 line ending
}

/// Build a full CSV document: a header row plus `rows`, every field guarded and
/// quoted. `rows` fields are borrowed, so the caller assembles owned strings and
/// passes `&str`s.
pub fn write_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    write_row(&mut out, headers.iter().copied());
    for row in rows {
        write_row(&mut out, row.iter().map(String::as_str));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_formula_triggers() {
        assert_eq!(sanitize_field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(sanitize_field("+1+1"), "'+1+1");
        assert_eq!(sanitize_field("-2+3"), "'-2+3");
        assert_eq!(sanitize_field("@cmd"), "'@cmd");
        assert_eq!(sanitize_field("\tfoo"), "'\tfoo");
        // Ordinary text is untouched (and stays borrowed).
        assert!(matches!(sanitize_field("great tool"), Cow::Borrowed(_)));
        assert_eq!(sanitize_field("great tool"), "great tool");
    }

    #[test]
    fn quotes_special_chars_and_doubles_quotes() {
        assert_eq!(encode_field("a,b"), "\"a,b\"");
        assert_eq!(encode_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(encode_field("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(encode_field("plain"), "plain");
    }

    #[test]
    fn injection_then_quoting_compose() {
        // A malicious anchor with both a trigger and a comma: guarded AND quoted.
        assert_eq!(encode_field("=1,2"), "\"'=1,2\"");
    }

    #[test]
    fn writes_a_document() {
        let csv = write_csv(
            &["source_url", "anchor"],
            &[
                vec!["https://a.com".into(), "great tool".into()],
                vec!["https://b.com".into(), "=HYPERLINK(\"x\")".into()],
            ],
        );
        let expected = "source_url,anchor\r\n\
             https://a.com,great tool\r\n\
             https://b.com,\"'=HYPERLINK(\"\"x\"\")\"\r\n";
        assert_eq!(csv, expected);
    }
}
