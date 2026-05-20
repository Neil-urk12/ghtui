/// Check if `query` fuzzy-matches `text`. Returns a score (higher = better match) or None.
pub fn fuzzy_match(query: &str, text: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    // Exact substring match gets highest score
    if let Some(pos) = text_lower.find(&query_lower) {
        return Some(100 - pos as i64);
    }

    // Fuzzy: each query char must appear in order
    let mut score = 0i64;
    let mut last_idx = 0;
    let text_chars: Vec<char> = text_lower.chars().collect();

    for qc in query_lower.chars() {
        let found = text_chars[last_idx..].iter().position(|&c| c == qc);
        match found {
            Some(pos) => {
                score += 10;
                // Bonus for consecutive matches
                if pos == 0 {
                    score += 5;
                }
                last_idx += pos + 1;
            }
            None => return None,
        }
    }

    // Bonus for shorter texts (more relevant)
    score += (100 - text.len() as i64).max(0);

    Some(score)
}

/// Filter a list of items by fuzzy matching on a string extracted from each item
pub fn fuzzy_filter<T, F>(items: &[T], query: &str, get_text: F) -> Vec<(usize, i64)>
where
    F: Fn(&T) -> String,
{
    let mut scored: Vec<(usize, i64)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let text = get_text(item);
            fuzzy_match(query, &text).map(|score| (i, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}
