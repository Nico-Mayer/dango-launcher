//! Case-insensitive fuzzy subsequence matching. Finds the best-scoring
//! alignment of the query within the text with a small dynamic program, and
//! returns the matched character positions so the frontend can highlight them.

pub struct Match {
    pub score: i32,
    pub positions: Vec<usize>,
}

const BASE: i32 = 2;
const CONSECUTIVE_BONUS: i32 = 7;
const WORD_START_BONUS: i32 = 9;
const LEADING_BONUS: i32 = 10;
const NEG: i32 = i32::MIN / 2;

/// `Some` when every character of `query` appears in `text` in order. An empty
/// query matches everything with a neutral score. The alignment maximises
/// contiguous runs and word-start hits while penalising gaps, so a prefix beats
/// a scattered match and a word-start beats a mid-word hit.
pub fn fuzzy_match(query: &str, text: &str) -> Option<Match> {
    let needle: Vec<char> = lower(query);
    if needle.is_empty() {
        return Some(Match {
            score: 0,
            positions: Vec::new(),
        });
    }
    let chars: Vec<char> = text.chars().collect();
    let lowered: Vec<char> = chars.iter().map(|c| first_lower(*c)).collect();
    let (n, m) = (needle.len(), lowered.len());
    if n > m {
        return None;
    }

    // score[i][j]: best score for aligning needle[..=i] with needle[i] landing
    // on text[j]. back[i][j]: the text position needle[i-1] landed on.
    let mut score = vec![vec![NEG; m]; n];
    let mut back = vec![vec![usize::MAX; m]; n];

    for j in 0..m {
        if lowered[j] == needle[0] {
            let mut s = BASE - j as i32;
            if is_word_start(&chars, j) {
                s += WORD_START_BONUS;
            }
            if j == 0 {
                s += LEADING_BONUS;
            }
            score[0][j] = s;
        }
    }

    for i in 1..n {
        // Running best of score[i-1][k] + k for k <= j-2, which folds the gap
        // penalty -(j-k-1) into an O(1) lookup as j advances.
        let mut best_prefix = NEG;
        let mut best_prefix_k = usize::MAX;
        for j in i..m {
            if j >= 2 {
                let k = j - 2;
                if k >= i - 1 && score[i - 1][k] > NEG {
                    let value = score[i - 1][k] + k as i32;
                    if value > best_prefix {
                        best_prefix = value;
                        best_prefix_k = k;
                    }
                }
            }
            if lowered[j] != needle[i] {
                continue;
            }
            let word_start = is_word_start(&chars, j);
            let mut best = NEG;
            let mut from = usize::MAX;

            // Consecutive: previous char landed right before this one.
            if score[i - 1][j - 1] > NEG {
                let mut cs = BASE + CONSECUTIVE_BONUS;
                if word_start {
                    cs += WORD_START_BONUS;
                }
                let candidate = score[i - 1][j - 1] + cs;
                if candidate > best {
                    best = candidate;
                    from = j - 1;
                }
            }
            // Gapped: best earlier landing, with the gap penalty folded in.
            if best_prefix > NEG {
                let mut cs = BASE;
                if word_start {
                    cs += WORD_START_BONUS;
                }
                let candidate = best_prefix - (j as i32 - 1) + cs;
                if candidate > best {
                    best = candidate;
                    from = best_prefix_k;
                }
            }
            if best > NEG {
                score[i][j] = best;
                back[i][j] = from;
            }
        }
    }

    let (end, &total) = score[n - 1]
        .iter()
        .enumerate()
        .filter(|(_, &s)| s > NEG)
        .max_by_key(|(_, &s)| s)?;

    let mut positions = vec![0usize; n];
    let mut j = end;
    for i in (0..n).rev() {
        positions[i] = j;
        if i > 0 {
            j = back[i][j];
        }
    }
    Some(Match {
        score: total,
        positions,
    })
}

fn is_word_start(chars: &[char], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    let prev = chars[pos - 1];
    let here = chars[pos];
    !prev.is_alphanumeric() || (here.is_uppercase() && !prev.is_uppercase())
}

fn lower(s: &str) -> Vec<char> {
    s.chars().map(first_lower).collect()
}

fn first_lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_prefix() {
        assert!(fuzzy_match("fire", "Firefox").is_some());
    }

    #[test]
    fn matches_a_gapped_subsequence() {
        let m = fuzzy_match("ffx", "Firefox").unwrap();
        assert_eq!(m.positions.len(), 3);
    }

    #[test]
    fn rejects_a_non_subsequence() {
        assert!(fuzzy_match("xyz", "Firefox").is_none());
    }

    #[test]
    fn is_case_insensitive() {
        assert!(fuzzy_match("CODE", "Visual Studio Code").is_some());
    }

    #[test]
    fn a_prefix_outscores_a_scattered_match() {
        let prefix = fuzzy_match("code", "Code Runner").unwrap();
        let scattered = fuzzy_match("code", "Console Detach Element").unwrap();
        assert!(
            prefix.score > scattered.score,
            "prefix {} vs scattered {}",
            prefix.score,
            scattered.score
        );
    }

    #[test]
    fn a_word_start_outscores_a_mid_word_hit() {
        let start = fuzzy_match("s", "Visual Studio").unwrap();
        let mid = fuzzy_match("s", "Basics").unwrap();
        assert!(
            start.score > mid.score,
            "start {} vs mid {}",
            start.score,
            mid.score
        );
    }

    #[test]
    fn finds_the_word_start_occurrence_not_the_first() {
        let m = fuzzy_match("s", "Visual Studio").unwrap();
        // "Studio" begins at index 7; the greedy first 's' would be index 2.
        assert_eq!(m.positions, vec![7]);
    }

    #[test]
    fn positions_point_at_matched_characters() {
        let m = fuzzy_match("vsc", "Visual Studio Code").unwrap();
        for &p in &m.positions {
            assert!(p < "Visual Studio Code".chars().count());
        }
    }
}
