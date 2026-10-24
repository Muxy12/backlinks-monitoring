//! CSV import parsing (§38). Pure: parse + column-map + per-row validate, no
//! I/O. Supports a generic layout and the common Ahrefs/Semrush export headers
//! via flexible header matching, so a user's existing export imports without
//! rebuilding it by hand (§49). The API layer bulk-creates backlinks from the
//! `ImportRow`s and reports the counts.

use serde::Serialize;

/// A validated import row, ready to become a backlink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportRow {
    pub row_number: usize,
    pub source_url: String,
    pub target_url: String,
    pub anchor: Option<String>,
    pub rel_type: String,
    pub cost: Option<String>,
    pub currency: Option<String>,
}

/// The outcome of one CSV data row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum RowResult {
    Ok(ImportRow),
    Error { row_number: usize, message: String },
}

/// Aggregate parse result.
#[derive(Debug, Clone, Serialize)]
pub struct ParseSummary {
    pub total: usize,
    pub rows: Vec<RowResult>,
}

impl ParseSummary {
    pub fn valid(&self) -> impl Iterator<Item = &ImportRow> {
        self.rows.iter().filter_map(|r| match r {
            RowResult::Ok(row) => Some(row),
            RowResult::Error { .. } => None,
        })
    }
    pub fn error_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| matches!(r, RowResult::Error { .. }))
            .count()
    }
}

fn norm_header(h: &str) -> String {
    h.trim().to_ascii_lowercase().replace(['_', '-'], " ")
}

/// Find the index of the first header matching any alias (exact, on normalized
/// headers).
fn find_col(headers: &[String], aliases: &[&str]) -> Option<usize> {
    headers.iter().position(|h| aliases.contains(&h.as_str()))
}

const SOURCE_ALIASES: &[&str] = &[
    "source url",
    "source",
    "url from",
    "referring page url",
    "referring page",
    "from",
    "sourceurl", // Majestic
];
const TARGET_ALIASES: &[&str] = &[
    "target url",
    "target",
    "url to",
    "link url",
    "destination url",
    "to",
    "targeturl", // Majestic
];
const ANCHOR_ALIASES: &[&str] = &[
    "anchor",
    "anchor text",
    "anchor and backlink",
    "anchortext", // Majestic
];
const REL_ALIASES: &[&str] = &["rel", "rel type", "type", "link type"];
// A boolean nofollow column (Majestic "FlagNoFollow": 1 = nofollow). Used only
// when no rel-word column is present.
const NOFOLLOW_FLAG_ALIASES: &[&str] = &["flagnofollow", "nofollow flag", "no follow"];
const COST_ALIASES: &[&str] = &["cost", "price", "paid"];
const CURRENCY_ALIASES: &[&str] = &["currency", "cur"];

fn is_http(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("http://") || s.starts_with("https://")
}

fn normalize_rel(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "dofollow" | "follow" => Some("dofollow".into()),
        "nofollow" => Some("nofollow".into()),
        "sponsored" => Some("sponsored".into()),
        "ugc" => Some("ugc".into()),
        _ => None,
    }
}

/// Parse CSV text into per-row results. The first record is the header row;
/// source and target columns are required.
pub fn parse_csv(input: &str) -> Result<ParseSummary, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("could not read header row: {e}"))?
        .iter()
        .map(norm_header)
        .collect();

    let src_i = find_col(&headers, SOURCE_ALIASES)
        .ok_or("no source URL column found (e.g. source_url, referring page URL)")?;
    let tgt_i = find_col(&headers, TARGET_ALIASES)
        .ok_or("no target URL column found (e.g. target_url, link URL)")?;
    let anchor_i = find_col(&headers, ANCHOR_ALIASES);
    let rel_i = find_col(&headers, REL_ALIASES);
    let flag_i = find_col(&headers, NOFOLLOW_FLAG_ALIASES);
    let cost_i = find_col(&headers, COST_ALIASES);
    let cur_i = find_col(&headers, CURRENCY_ALIASES);

    let get = |rec: &csv::StringRecord, i: Option<usize>| {
        i.and_then(|i| rec.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let mut rows = Vec::new();
    for (idx, rec) in reader.records().enumerate() {
        let row_number = idx + 2; // 1-based, +1 for the header row
        let rec = match rec {
            Ok(r) => r,
            Err(e) => {
                rows.push(RowResult::Error {
                    row_number,
                    message: format!("malformed row: {e}"),
                });
                continue;
            }
        };

        let source = rec.get(src_i).unwrap_or("").trim().to_string();
        let target = rec.get(tgt_i).unwrap_or("").trim().to_string();
        if !is_http(&source) {
            rows.push(RowResult::Error {
                row_number,
                message: "source URL is missing or not http(s)".into(),
            });
            continue;
        }
        if !is_http(&target) {
            rows.push(RowResult::Error {
                row_number,
                message: "target URL is missing or not http(s)".into(),
            });
            continue;
        }
        // rel precedence: an explicit rel-word column, else a nofollow-flag
        // column (Majestic "FlagNoFollow"), else default dofollow.
        let rel_type = if rel_i.is_some() {
            let rel = get(&rec, rel_i).unwrap_or_default();
            match normalize_rel(&rel) {
                Some(r) => r,
                None => {
                    rows.push(RowResult::Error {
                        row_number,
                        message: format!("unknown rel type {rel:?}"),
                    });
                    continue;
                }
            }
        } else if let Some(fi) = flag_i {
            let v = rec.get(fi).unwrap_or("").trim().to_ascii_lowercase();
            if matches!(v.as_str(), "1" | "true" | "yes" | "y") {
                "nofollow".into()
            } else {
                "dofollow".into()
            }
        } else {
            "dofollow".into()
        };
        let cost = get(&rec, cost_i);
        // A cost needs a currency to be meaningful (matches the DB CHECK).
        let currency = get(&rec, cur_i).map(|c| c.to_ascii_uppercase());
        if cost.is_some() && currency.is_none() {
            rows.push(RowResult::Error {
                row_number,
                message: "cost present but no currency column/value".into(),
            });
            continue;
        }

        rows.push(RowResult::Ok(ImportRow {
            row_number,
            source_url: source,
            target_url: target,
            anchor: get(&rec, anchor_i),
            rel_type,
            cost,
            currency,
        }));
    }

    Ok(ParseSummary {
        total: rows.len(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_rows(s: &ParseSummary) -> Vec<&ImportRow> {
        s.valid().collect()
    }

    #[test]
    fn parses_a_generic_export() {
        let csv = "source_url,target_url,anchor,rel_type,cost,currency\n\
                   https://blog.com/a,https://me.com/x,great tool,dofollow,120,EUR\n\
                   https://news.io/b,https://me.com/y,,nofollow,,\n";
        let s = parse_csv(csv).unwrap();
        assert_eq!(s.total, 2);
        let rows = ok_rows(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source_url, "https://blog.com/a");
        assert_eq!(rows[0].anchor.as_deref(), Some("great tool"));
        assert_eq!(rows[0].cost.as_deref(), Some("120"));
        assert_eq!(rows[0].currency.as_deref(), Some("EUR"));
        assert_eq!(rows[1].rel_type, "nofollow");
        assert_eq!(rows[1].cost, None);
    }

    #[test]
    fn matches_ahrefs_style_headers() {
        // Ahrefs: "Referring page URL" / "Link URL" / "Anchor".
        let csv = "Referring page URL,Link URL,Anchor,Type\n\
                   https://ref.com/p,https://me.com/z,click here,dofollow\n";
        let s = parse_csv(csv).unwrap();
        let rows = ok_rows(&s);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source_url, "https://ref.com/p");
        assert_eq!(rows[0].target_url, "https://me.com/z");
        assert_eq!(rows[0].anchor.as_deref(), Some("click here"));
    }

    #[test]
    fn matches_majestic_style_headers_with_nofollow_flag() {
        // Majestic: "SourceURL" / "TargetURL" / "AnchorText" + "FlagNoFollow" (0/1).
        let csv = "SourceURL,TargetURL,AnchorText,FlagNoFollow\n\
                   https://ref.com/a,https://me.com/x,great tool,0\n\
                   https://spam.io/b,https://me.com/y,cheap links,1\n";
        let s = parse_csv(csv).unwrap();
        let rows = ok_rows(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source_url, "https://ref.com/a");
        assert_eq!(rows[0].target_url, "https://me.com/x");
        assert_eq!(rows[0].anchor.as_deref(), Some("great tool"));
        assert_eq!(rows[0].rel_type, "dofollow", "FlagNoFollow=0 -> dofollow");
        assert_eq!(rows[1].rel_type, "nofollow", "FlagNoFollow=1 -> nofollow");
    }

    #[test]
    fn missing_required_column_is_an_error() {
        let csv = "anchor,rel\nfoo,dofollow\n";
        assert!(
            parse_csv(csv).is_err(),
            "no source/target columns -> parse error"
        );
    }

    #[test]
    fn bad_rows_are_reported_but_do_not_stop_the_import() {
        let csv = "source_url,target_url,rel_type\n\
                   https://ok.com/a,https://me.com/x,dofollow\n\
                   not-a-url,https://me.com/y,dofollow\n\
                   https://ok.com/c,https://me.com/z,weirdrel\n";
        let s = parse_csv(csv).unwrap();
        assert_eq!(s.total, 3);
        assert_eq!(ok_rows(&s).len(), 1, "only the first row is valid");
        assert_eq!(s.error_count(), 2);
    }

    #[test]
    fn cost_without_currency_is_rejected() {
        let csv = "source_url,target_url,cost\nhttps://a.com/x,https://me.com/y,50\n";
        let s = parse_csv(csv).unwrap();
        assert_eq!(s.error_count(), 1);
    }

    #[test]
    fn row_numbers_account_for_the_header() {
        let csv = "source_url,target_url\nbad,bad\n";
        let s = parse_csv(csv).unwrap();
        match &s.rows[0] {
            RowResult::Error { row_number, .. } => assert_eq!(*row_number, 2),
            _ => panic!("expected error"),
        }
    }
}
