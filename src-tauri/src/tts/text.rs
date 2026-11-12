//! Text preparation for speech: cleaning, splitting into speakable chunks,
//! and a pronunciation guess for words the dictionary doesn't know.

/// The model accepts about 510 phonemes at a time, and a phoneme is roughly
/// one letter, so chunks stay well below that.
const MAX_CHUNK_CHARS: usize = 300;

/// Split text into chunks that are spoken one after another. Chunks end at
/// sentence boundaries where possible, so the voice pauses naturally and the
/// first words can start playing while the rest is still being made.
pub fn chunks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for sentence in sentences(&clean(text)) {
        for piece in split_long(&sentence) {
            // Skip pieces with nothing to say (stray punctuation).
            if piece.chars().any(char::is_alphanumeric) {
                out.push(piece);
            }
        }
    }
    out
}

/// Drop formatting that should not be read out, and turn line breaks into
/// sentence breaks.
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line
            .trim()
            .trim_start_matches(['#', '>', '-', '*', '•'])
            .trim();
        if line.is_empty() {
            continue;
        }
        let line: String = line
            .chars()
            .filter(|c| !matches!(c, '*' | '`' | '_' | '#' | '~' | '|'))
            .collect();
        out.push_str(line.trim());
        // A heading or list item without closing punctuation still ends a thought.
        if !line.trim_end().ends_with(['.', '!', '?', ':', ';', ',']) {
            out.push('.');
        }
        out.push(' ');
    }
    out
}

fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        current.push(c);
        // A sentence ends at . ! ? followed by a space, so "3.5" stays whole.
        // A full stop after a title or an initial ("Dr. Smith", "J. Doe")
        // does not end it.
        let ends = matches!(c, '.' | '!' | '?')
            && chars.peek().is_none_or(|n| n.is_whitespace())
            && !(c == '.' && ends_with_abbreviation(&current));
        if ends {
            let done = std::mem::take(&mut current);
            if !done.trim().is_empty() {
                out.push(done.trim().to_string());
            }
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

fn ends_with_abbreviation(sentence: &str) -> bool {
    const ABBREVIATIONS: &[&str] = &[
        "dr", "mr", "mrs", "ms", "prof", "sr", "jr", "st", "vs", "etc", "e.g", "i.e", "no", "inc",
        "ltd", "co",
    ];
    let word = sentence
        .trim_end_matches('.')
        .rsplit(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("");
    // "No." and "Co." are abbreviations only when capitalised mid-sentence;
    // at the end of a sentence they are rare enough to accept.
    let is_initial = word.chars().count() == 1 && word.chars().all(char::is_uppercase);
    is_initial || ABBREVIATIONS.contains(&word.to_lowercase().as_str())
}

/// Break an over-long sentence at commas, or failing that at spaces.
fn split_long(sentence: &str) -> Vec<String> {
    if sentence.chars().count() <= MAX_CHUNK_CHARS {
        return vec![sentence.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in sentence.split_inclusive(' ') {
        let too_long = current.chars().count() + word.chars().count() > MAX_CHUNK_CHARS;
        let at_comma = current.trim_end().ends_with([',', ';', ':'])
            && current.chars().count() > MAX_CHUNK_CHARS / 2;
        if !current.is_empty() && (too_long || at_comma) {
            out.push(std::mem::take(&mut current).trim().to_string());
        }
        current.push_str(word);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// Guess how to say a word that is not in the pronunciation dictionary
/// (product names, usernames, slang). Short all-caps words and words without
/// vowels are spelled out; the rest go through simple English spelling rules.
/// The result uses the same phoneme alphabet as the dictionary.
pub fn guess_pronunciation(word: &str) -> String {
    let letters: Vec<char> = word.chars().filter(char::is_ascii_alphabetic).collect();
    if letters.is_empty() {
        return String::new();
    }
    let all_caps = letters.iter().all(char::is_ascii_uppercase);
    let w: Vec<char> = letters.iter().map(char::to_ascii_lowercase).collect();
    let has_vowel = w.iter().any(|c| "aeiouy".contains(*c));
    if (all_caps && w.len() <= 5) || !has_vowel {
        return w.iter().map(|&c| letter_name(c)).collect();
    }
    sound_out(&w)
}

fn letter_name(c: char) -> &'static str {
    match c {
        'a' => "ˈA", 'b' => "bˈi", 'c' => "sˈi", 'd' => "dˈi", 'e' => "ˈi", 'f' => "ˈɛf",
        'g' => "ʤˈi", 'h' => "ˈAʧ", 'i' => "ˈI", 'j' => "ʤˈA", 'k' => "kˈA", 'l' => "ˈɛl",
        'm' => "ˈɛm", 'n' => "ˈɛn", 'o' => "ˈO", 'p' => "pˈi", 'q' => "kjˈu", 'r' => "ˈɑɹ",
        's' => "ˈɛs", 't' => "tˈi", 'u' => "jˈu", 'v' => "vˈi", 'w' => "dˈʌbᵊljˌu",
        'x' => "ˈɛks", 'y' => "wˈI", _ => "zˈi",
    }
}

/// Letter groups and their usual sound, longest first.
const GROUPS: &[(&str, &str)] = &[
    ("tion", "ʃən"), ("sion", "ʒən"), ("ough", "O"), ("igh", "I"), ("tch", "ʧ"),
    ("sch", "sk"), ("ch", "ʧ"), ("sh", "ʃ"), ("th", "θ"), ("ph", "f"), ("wh", "w"),
    ("ck", "k"), ("ng", "ŋ"), ("qu", "kw"), ("ee", "i"), ("ea", "i"), ("oo", "u"),
    ("ou", "W"), ("ow", "O"), ("oi", "Y"), ("oy", "Y"), ("ai", "A"), ("ay", "A"),
    ("au", "ɔ"), ("aw", "ɔ"), ("er", "əɹ"), ("ir", "ɜɹ"), ("ur", "ɜɹ"), ("ar", "ɑɹ"),
    ("or", "ɔɹ"),
];

fn is_vowel(c: char) -> bool {
    "aeiou".contains(c)
}

fn sound_out(w: &[char]) -> String {
    let n = w.len();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < n {
        let c = w[i];
        let rest: String = w[i..].iter().collect();

        if let Some((group, sound)) = GROUPS.iter().find(|(g, _)| rest.starts_with(g)) {
            out.push(sound);
            i += group.len();
            continue;
        }
        // Doubled consonants are said once.
        if i > 0 && w[i - 1] == c && !is_vowel(c) {
            i += 1;
            continue;
        }
        let last = i == n - 1;
        let next = w.get(i + 1).copied();
        // "Vowel, consonant, final e" makes the vowel long: make, time, tone.
        let magic_e = i + 3 == n && w[n - 1] == 'e' && next.is_some_and(|x| !is_vowel(x));
        let sound = match c {
            'a' if magic_e => "A",
            'e' if magic_e => "i",
            'i' if magic_e => "I",
            'o' if magic_e => "O",
            'u' if magic_e => "ju",
            'a' if last => "ə",
            'a' => "æ",
            // A final e is silent when the word has another vowel.
            'e' if last && n > 2 && w[..n - 1].iter().any(|&x| is_vowel(x) || x == 'y') => "",
            'e' => "ɛ",
            'i' if last => "i",
            'i' => "ɪ",
            'o' if last => "O",
            'o' => "ɑ",
            'u' => "ʌ",
            'y' if i == 0 => "j",
            'y' if last && n <= 3 => "I",
            'y' if last => "i",
            'y' => "ɪ",
            'c' if next.is_some_and(|x| "eiy".contains(x)) => "s",
            'c' | 'q' => "k",
            'g' if next == Some('e') && i + 2 == n => "ʤ",
            'g' => "ɡ",
            'j' => "ʤ",
            'x' => "ks",
            // A final r after a consonant gets a vowel: "Loudr", "Tumblr".
            'r' if last && i > 0 && !is_vowel(w[i - 1]) => "əɹ",
            'r' => "ɹ",
            'b' => "b", 'd' => "d", 'f' => "f", 'h' => "h", 'k' => "k", 'l' => "l",
            'm' => "m", 'n' => "n", 'p' => "p", 's' => "s", 't' => "t", 'v' => "v",
            'w' => "w", 'z' => "z",
            _ => "",
        };
        out.push(sound);
        i += 1;
    }

    // Stress the first vowel sound.
    const VOWEL_SOUNDS: &str = "AIOWYiuæɑɔɛɜɪʊʌə";
    let first_vowel = out
        .iter()
        .position(|s| s.chars().next().is_some_and(|c| VOWEL_SOUNDS.contains(c)))
        // "ju" (long u) starts with a consonant sound; stress goes before it.
        .or_else(|| out.iter().position(|s| *s == "ju"));
    let mut result = String::new();
    for (index, sound) in out.iter().enumerate() {
        if Some(index) == first_vowel {
            result.push('ˈ');
        }
        result.push_str(sound);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sentences_and_headings() {
        let c = chunks("# Title\n\nFirst one. Second one!\n- item");
        assert_eq!(c, ["Title.", "First one.", "Second one!", "item."]);
    }

    #[test]
    fn titles_do_not_end_a_sentence() {
        let c = chunks("Dr. Smith met J. Doe at 3.5 km. They talked.");
        assert_eq!(c, ["Dr. Smith met J. Doe at 3.5 km.", "They talked."]);
    }

    #[test]
    fn long_sentences_are_split() {
        let long = "word ".repeat(200);
        let c = chunks(&long);
        assert!(c.len() > 2);
        assert!(c.iter().all(|p| p.chars().count() <= MAX_CHUNK_CHARS));
    }

    #[test]
    fn guesses_names() {
        assert_eq!(guess_pronunciation("Loudr"), "lˈWdəɹ");
        assert_eq!(guess_pronunciation("Wispr"), "wˈɪspəɹ");
        assert_eq!(guess_pronunciation("GPT"), "ʤˈipˈitˈi");
        assert_eq!(guess_pronunciation("zoomify"), "zˈumɪfi");
    }
}
