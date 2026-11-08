//! Heuristic site profiling (refonte P2). Pure: a site's own visible text
//! (title, meta description, headings) in, its most salient terms out. This is a
//! cheap baseline used until an LLM is configured — it is NOT a semantic
//! analysis, so the worker tags its result `source = "heuristic"` and leaves the
//! themed fields (sub-themes, products) empty rather than guessing them.

/// Common French + English stop-words to drop before counting. Deliberately
/// small — enough to cut the obvious noise, not a full NLP list.
const STOP: &[&str] = &[
    // French
    "les", "des", "une", "nos", "vos", "pour", "avec", "dans", "sur", "par", "aux", "est", "son",
    "ses", "leur", "leurs", "qui", "que", "pas", "plus", "tout", "tous", "cette", "votre", "notre",
    "chez", "mais", "ou", "et", "un", "le", "la", "de", "du", "en", "au", "ce", "il", "elle",
    // English
    "the", "and", "for", "with", "your", "our", "you", "are", "all", "from", "this", "that", "has",
    "can", "will", "more", "about", "home", "page", "best", "new", "get", "how", "why", "what",
];

fn is_stop(w: &str) -> bool {
    STOP.contains(&w)
}

/// The most salient terms across the given texts: lowercased single words,
/// stop-words / short / numeric tokens dropped, ranked by frequency (ties broken
/// alphabetically for a stable result), capped at `max`.
pub fn salient_terms(texts: &[String], max: usize) -> Vec<String> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, u32> = HashMap::new();
    for t in texts {
        for raw in t.split(|c: char| !c.is_alphanumeric()) {
            let w = raw.to_lowercase();
            if w.chars().count() < 3 || w.chars().all(|c| c.is_numeric()) || is_stop(&w) {
                continue;
            }
            *counts.entry(w).or_insert(0) += 1;
        }
    }
    let mut ranked: Vec<(String, u32)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.into_iter().take(max).map(|(w, _)| w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_by_frequency_and_drops_noise() {
        let texts = vec![
            "Pulvérisateur de jardin — le meilleur pulvérisateur".to_string(),
            "Pulvérisateur électrique et pulvérisateur à pression".to_string(),
            "Guide 2024 des pulvérisateurs pour le jardin".to_string(),
        ];
        let terms = salient_terms(&texts, 5);
        assert_eq!(terms.first().map(String::as_str), Some("pulvérisateur"));
        assert!(terms.iter().any(|t| t == "jardin"));
        // stop-words, short tokens and the year are gone.
        assert!(!terms.iter().any(|t| t == "le" || t == "de" || t == "2024"));
    }

    #[test]
    fn empty_in_empty_out() {
        assert!(salient_terms(&[], 10).is_empty());
        assert!(salient_terms(&["le de la les".to_string()], 10).is_empty());
    }
}
