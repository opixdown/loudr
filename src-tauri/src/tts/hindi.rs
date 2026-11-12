//! Pronunciation for the Hindi voices: Devanagari, and Hindi typed in
//! English letters ("Hinglish"), mixed freely with English words.
//!
//! Devanagari is close to phonetic, so words are sounded out by rule: map
//! each letter, drop the unwritten "a" where speakers drop it, then place the
//! stress. Romanised Hindi has no fixed spelling, so common words are looked
//! up (and converted through their Devanagari spelling) and the rest are
//! sounded out with the usual romanisation habits. English words are left to
//! the English dictionary.
//!
//! The phoneme spellings follow what the voice model was trained on, which is
//! why some look unusual ("c" for च, "r." for ड़).

/// Turn one chunk of text into phonemes for a Hindi voice. `english` reads a
/// run of English words.
pub fn phonemize(text: &str, mut english: impl FnMut(&str) -> String) -> String {
    let tokens = tokenize(text);
    let hindi_at = hinglish_words(&tokens);

    let mut out = String::new();
    // English words and what lies between them, collected so the English
    // reader sees whole phrases.
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String, english: &mut dyn FnMut(&str) -> String| {
        if !run.chars().any(char::is_alphanumeric) {
            out.push_str(run);
        } else {
            let lead = run.len() - run.trim_start().len();
            let trail = run.trim_end().len();
            out.push_str(&run[..lead]);
            out.push_str(english(run[lead..trail].trim()).trim());
            out.push_str(&run[trail..]);
        }
        run.clear();
    };

    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Devanagari(word) => {
                flush(&mut run, &mut out, &mut english);
                out.push_str(&devanagari_word(word));
            }
            Token::Latin(word) if hindi_at[index] => {
                flush(&mut run, &mut out, &mut english);
                let spelling = lexicon(&word.to_lowercase()).map(|(devanagari, _)| devanagari);
                out.push_str(&spelling.map_or_else(|| roman_word(word), devanagari_word));
            }
            Token::Latin(word) => run.push_str(word),
            Token::Other(text) => {
                // Sentence punctuation between two Hindi words is kept as is;
                // inside an English run it stays with the run.
                for c in text.chars() {
                    run.push(if matches!(c, '।' | '॥') { '.' } else { c });
                }
            }
        }
    }
    flush(&mut run, &mut out, &mut english);
    out
}

/// Respell English phonemes with the sounds the Hindi voices know, the way
/// English is spoken in India: "t" and "d" are retroflex, "r" is tapped,
/// "w" and "v" are one sound, and the gliding vowels are plain.
pub fn indian_english(phonemes: &str) -> String {
    let mut out = String::with_capacity(phonemes.len());
    for c in phonemes.chars() {
        match c {
            'A' => out.push_str("eː"),
            'I' => out.push_str("aɪ"),
            'O' => out.push_str("oː"),
            'W' => out.push_str("aʊ"),
            'Y' => out.push_str("ɔɪ"),
            'ʤ' => out.push('ɟ'),
            'ʧ' => out.push('c'),
            'ɹ' => out.push('ɾ'),
            't' | 'T' => out.push('ʈ'),
            'd' => out.push('ɖ'),
            'θ' => out.push_str("tʰ"),
            'ð' => out.push('d'),
            'w' | 'v' => out.push('ʋ'),
            'æ' => out.push('ɛ'),
            'ᵊ' | 'ɜ' => out.push('ə'),
            'ɑ' => out.push_str("aː"),
            other => out.push(other),
        }
    }
    out
}

enum Token<'a> {
    Devanagari(&'a str),
    Latin(&'a str),
    /// Spaces, digits and punctuation.
    Other(&'a str),
}

fn is_devanagari(c: char) -> bool {
    ('\u{0900}'..='\u{097F}').contains(&c) && !matches!(c, '।' | '॥') && !c.is_numeric()
}

fn tokenize(text: &str) -> Vec<Token<'_>> {
    #[derive(PartialEq, Clone, Copy)]
    enum Kind {
        Devanagari,
        Latin,
        Other,
    }
    let kind = |c: char| {
        if is_devanagari(c) {
            Kind::Devanagari
        } else if c.is_ascii_alphabetic() || c == '\'' {
            Kind::Latin
        } else {
            Kind::Other
        }
    };
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut current: Option<Kind> = None;
    for (i, c) in text.char_indices() {
        let k = kind(c);
        if current != Some(k) {
            if let Some(previous) = current {
                tokens.push((previous, &text[start..i]));
            }
            start = i;
            current = Some(k);
        }
    }
    if let Some(previous) = current {
        tokens.push((previous, &text[start..]));
    }
    tokens
        .into_iter()
        .map(|(k, s)| match k {
            Kind::Devanagari => Token::Devanagari(s),
            Kind::Latin => Token::Latin(s),
            Kind::Other => Token::Other(s),
        })
        .collect()
}

/// For each token, whether it is a Latin-script word to read as Hindi.
///
/// Many romanised Hindi words are also English words ("to", "main", "the").
/// Those are read as Hindi only next to a word that is clearly Hindi.
fn hinglish_words(tokens: &[Token]) -> Vec<bool> {
    // Word tokens in order, with their position in `tokens`.
    let words: Vec<(usize, Option<bool>)> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| match t {
            Token::Latin(w) => Some((i, lexicon(&w.to_lowercase()).map(|(_, shared)| shared))),
            Token::Devanagari(_) => Some((i, Some(false))),
            Token::Other(_) => None,
        })
        .collect();
    let clearly_hindi = |entry: Option<&(usize, Option<bool>)>| matches!(entry, Some((_, Some(false))));

    let mut out = vec![false; tokens.len()];
    for (position, (index, entry)) in words.iter().enumerate() {
        let Token::Latin(word) = &tokens[*index] else { continue };
        out[*index] = match entry {
            Some(false) => true,
            Some(true) => {
                let before = position.checked_sub(1).and_then(|p| words.get(p));
                let after = words.get(position + 1);
                // "the" is Hindi (थे) only after a Hindi word: English "the"
                // comes before its noun.
                if word.eq_ignore_ascii_case("the") {
                    clearly_hindi(before)
                } else {
                    clearly_hindi(before) || clearly_hindi(after)
                }
            }
            None => false,
        };
    }
    out
}

/// Common Hindi words as typed in English letters: (Devanagari spelling,
/// whether the same letters are also an English word).
fn lexicon(word: &str) -> Option<(&'static str, bool)> {
    LEXICON
        .iter()
        .find(|(roman, _, _)| *roman == word)
        .map(|(_, devanagari, shared)| (*devanagari, *shared))
}

#[rustfmt::skip]
const LEXICON: &[(&str, &str, bool)] = &[
    // Shared with English.
    ("to", "तो", true), ("the", "थे", true), ("main", "मैं", true), ("par", "पर", true),
    ("is", "इस", true), ("us", "उस", true), ("in", "इन", true), ("me", "में", true),
    ("hi", "ही", true), ("ho", "हो", true), ("do", "दो", true), ("hum", "हम", true),
    ("din", "दिन", true), ("log", "लोग", true), ("are", "अरे", true), ("ab", "अब", true),
    ("jab", "जब", true), ("tab", "तब", true), ("man", "मन", true), ("mat", "मत", true),
    ("pass", "पास", true), ("ya", "या", true), ("fir", "फिर", true), ("le", "ले", true),
    ("de", "दे", true), ("char", "चार", true), ("sham", "शाम", true), ("bas", "बस", false),
    ("bat", "बात", true), ("kam", "कम", true), ("bad", "बाद", true),
    ("pe", "पे", true), ("so", "सो", true), ("ban", "बन", true), ("bit", "बीत", true),
    // Pronouns and helpers.
    ("hai", "है", false), ("hain", "हैं", false), ("hun", "हूँ", false), ("hoon", "हूँ", false),
    ("tha", "था", false), ("thi", "थी", false), ("hoga", "होगा", false), ("hogi", "होगी", false),
    ("honge", "होंगे", false), ("hua", "हुआ", false), ("hui", "हुई", false), ("hue", "हुए", false),
    ("mai", "मैं", false), ("mein", "में", false), ("mei", "में", false), ("tum", "तुम", false),
    ("aap", "आप", false), ("ham", "हम", false), ("ye", "ये", false), ("yeh", "यह", false),
    ("wo", "वो", false), ("woh", "वह", false), ("vo", "वो", false), ("mera", "मेरा", false),
    ("meri", "मेरी", false), ("mere", "मेरे", false), ("tera", "तेरा", false), ("teri", "तेरी", false),
    ("tere", "तेरे", false), ("tumhara", "तुम्हारा", false), ("tumhari", "तुम्हारी", false),
    ("aapka", "आपका", false), ("aapki", "आपकी", false), ("aapke", "आपके", false),
    ("uska", "उसका", false), ("uski", "उसकी", false), ("uske", "उसके", false),
    ("iska", "इसका", false), ("iski", "इसकी", false), ("iske", "इसके", false),
    ("unka", "उनका", false), ("unki", "उनकी", false), ("unke", "उनके", false),
    ("hamara", "हमारा", false), ("hamari", "हमारी", false), ("humara", "हमारा", false),
    ("apna", "अपना", false), ("apni", "अपनी", false), ("apne", "अपने", false),
    ("mujhe", "मुझे", false), ("mujhko", "मुझको", false), ("tujhe", "तुझे", false),
    ("tumhe", "तुम्हें", false), ("hume", "हमें", false), ("humein", "हमें", false),
    ("hamein", "हमें", false), ("unhe", "उन्हें", false),
    ("isse", "इससे", false), ("usse", "उससे", false), ("ise", "इसे", false),
    ("ka", "का", false), ("ki", "की", false), ("ke", "के", false), ("ko", "को", false),
    ("se", "से", false), ("ne", "ने", false), ("tak", "तक", false), ("bhi", "भी", false),
    ("toh", "तो", false), ("na", "ना", false), ("liye", "लिए", false), ("saath", "साथ", false),
    ("sath", "साथ", false), ("bina", "बिना", false), ("baad", "बाद", false),
    ("pehle", "पहले", false), ("pahle", "पहले", false), ("upar", "ऊपर", false),
    ("neeche", "नीचे", false), ("niche", "नीचे", true), ("andar", "अंदर", false),
    ("bahar", "बाहर", false), ("paas", "पास", false), ("wala", "वाला", false),
    ("wali", "वाली", false), ("wale", "वाले", false),
    // Questions.
    ("kya", "क्या", false), ("kyu", "क्यों", false), ("kyun", "क्यों", false),
    ("kyon", "क्यों", false), ("kaise", "कैसे", false), ("kaisa", "कैसा", false),
    ("kaisi", "कैसी", false), ("kahan", "कहाँ", false), ("kaha", "कहाँ", false),
    ("yahan", "यहाँ", false), ("yaha", "यहाँ", false), ("wahan", "वहाँ", false),
    ("waha", "वहाँ", false), ("kab", "कब", false), ("kaun", "कौन", false),
    ("kitna", "कितना", false), ("kitni", "कितनी", false), ("kitne", "कितने", false),
    ("kis", "किस", false), ("kisi", "किसी", false), ("kuch", "कुछ", false),
    ("kuchh", "कुछ", false), ("koi", "कोई", false), ("sab", "सब", false),
    ("sabse", "सबसे", false), ("sabhi", "सभी", false), ("dono", "दोनों", false),
    // Joining words and adverbs.
    ("aur", "और", false), ("lekin", "लेकिन", false), ("magar", "मगर", false),
    ("phir", "फिर", false), ("agar", "अगर", false), ("kyunki", "क्योंकि", false),
    ("kyonki", "क्योंकि", false), ("isliye", "इसलिए", false), ("isiliye", "इसीलिए", false),
    ("ki", "कि", false), ("haan", "हाँ", false), ("han", "हाँ", false),
    ("nahi", "नहीं", false), ("nahin", "नहीं", false), ("nhi", "नहीं", false),
    ("bahut", "बहुत", false), ("bohot", "बहुत", false), ("bahot", "बहुत", false),
    ("thoda", "थोड़ा", false), ("thodi", "थोड़ी", false), ("zyada", "ज़्यादा", false),
    ("jyada", "ज़्यादा", false), ("itna", "इतना", false), ("itni", "इतनी", false),
    ("abhi", "अभी", false), ("kabhi", "कभी", false), ("hamesha", "हमेशा", false),
    ("sirf", "सिर्फ़", false), ("bilkul", "बिल्कुल", false), ("zaroor", "ज़रूर", false),
    ("zarur", "ज़रूर", false), ("shayad", "शायद", false), ("jaldi", "जल्दी", false),
    ("aaj", "आज", false), ("kal", "कल", false), ("parso", "परसों", false),
    ("subah", "सुबह", false), ("shaam", "शाम", false), ("raat", "रात", false),
    ("accha", "अच्छा", false), ("acha", "अच्छा", false), ("achha", "अच्छा", false),
    ("acchi", "अच्छी", false), ("achi", "अच्छी", false), ("acche", "अच्छे", false),
    ("theek", "ठीक", false), ("thik", "ठीक", false), ("sahi", "सही", false),
    ("galat", "ग़लत", false), ("bura", "बुरा", false), ("bada", "बड़ा", false),
    ("badi", "बड़ी", false), ("bade", "बड़े", false), ("chhota", "छोटा", false),
    ("chota", "छोटा", false), ("choti", "छोटी", false), ("naya", "नया", false),
    ("nayi", "नई", false), ("naye", "नए", false), ("purana", "पुराना", false),
    ("sundar", "सुंदर", false), ("mushkil", "मुश्किल", false), ("aasan", "आसान", false),
    ("asaan", "आसान", false), ("khush", "ख़ुश", false), ("pura", "पूरा", false),
    ("poora", "पूरा", false), ("puri", "पूरी", false),
    // Verbs.
    ("kar", "कर", false), ("karna", "करना", false), ("karo", "करो", false),
    ("kiya", "किया", false), ("kiye", "किए", false), ("karta", "करता", false),
    ("karti", "करती", false), ("karte", "करते", false), ("karenge", "करेंगे", false),
    ("karunga", "करूँगा", false), ("karungi", "करूँगी", false), ("karega", "करेगा", false),
    ("karegi", "करेगी", false), ("kare", "करे", false), ("karein", "करें", false),
    ("raha", "रहा", false), ("rahi", "रही", false), ("rahe", "रहे", false),
    ("gaya", "गया", false), ("gayi", "गई", false), ("gaye", "गए", false),
    ("aaya", "आया", false), ("aayi", "आई", false), ("aaye", "आए", false),
    ("jana", "जाना", false), ("jaana", "जाना", false), ("jao", "जाओ", false),
    ("jata", "जाता", false), ("jati", "जाती", false), ("jate", "जाते", false),
    ("jaa", "जा", false), ("aana", "आना", false), ("aao", "आओ", false),
    ("aaja", "आजा", false), ("aata", "आता", false), ("aati", "आती", false),
    ("dekho", "देखो", false), ("dekh", "देख", false), ("dekha", "देखा", false),
    ("dekhna", "देखना", false), ("suno", "सुनो", false), ("suna", "सुना", false),
    ("sunna", "सुनना", false), ("bolo", "बोलो", false), ("bola", "बोला", false),
    ("bol", "बोल", false), ("bolna", "बोलना", false), ("chalo", "चलो", false),
    ("chal", "चल", false), ("chalna", "चलना", false), ("chalta", "चलता", false),
    ("lo", "लो", false), ("lena", "लेना", false), ("liya", "लिया", false),
    ("dena", "देना", false), ("diya", "दिया", false), ("dedo", "दे दो", false),
    ("mila", "मिला", false), ("milta", "मिलता", false), ("milna", "मिलना", false),
    ("milte", "मिलते", false), ("chahiye", "चाहिए", false), ("chahta", "चाहता", false),
    ("chahti", "चाहती", false), ("sakta", "सकता", false), ("sakti", "सकती", false),
    ("sakte", "सकते", false), ("pata", "पता", false), ("lagta", "लगता", false),
    ("lagti", "लगती", false), ("laga", "लगा", false), ("samajh", "समझ", false),
    ("samjha", "समझा", false), ("batao", "बताओ", false), ("bata", "बता", false),
    ("bataya", "बताया", false), ("khana", "खाना", false), ("khao", "खाओ", false),
    ("piyo", "पियो", false), ("likho", "लिखो", false), ("likhna", "लिखना", false),
    ("padh", "पढ़", false), ("padhke", "पढ़के", false), ("padha", "पढ़ा", false), ("padhai", "पढ़ाई", false), ("padho", "पढ़ो", false), ("padhna", "पढ़ना", false), ("socho", "सोचो", false),
    ("socha", "सोचा", false), ("ruko", "रुको", false), ("baitho", "बैठो", false),
    ("rakho", "रखो", false), ("rakh", "रख", false), ("banao", "बनाओ", false),
    ("banaya", "बनाया", false), ("banana", "बनाना", true), ("hona", "होना", false),
    ("hota", "होता", false), ("hoti", "होती", false), ("hote", "होते", false),
    // Nouns and greetings.
    ("bhai", "भाई", false), ("yaar", "यार", false), ("dost", "दोस्त", false),
    ("ghar", "घर", false), ("kaam", "काम", false), ("naam", "नाम", false),
    ("baat", "बात", false), ("saal", "साल", false), ("paisa", "पैसा", false),
    ("paise", "पैसे", false), ("pani", "पानी", false), ("paani", "पानी", false),
    ("cheez", "चीज़", false), ("chiz", "चीज़", false), ("samay", "समय", false),
    ("waqt", "वक़्त", false), ("matlab", "मतलब", false), ("ek", "एक", false),
    ("teen", "तीन", true), ("paanch", "पाँच", false), ("das", "दस", false),
    ("namaste", "नमस्ते", false), ("namaskar", "नमस्कार", false),
    ("shukriya", "शुक्रिया", false), ("dhanyavaad", "धन्यवाद", false),
    ("dhanyawad", "धन्यवाद", false), ("dhanyavad", "धन्यवाद", false),
    ("kripya", "कृपया", false), ("maaf", "माफ़", false), ("pyaar", "प्यार", false),
    ("pyar", "प्यार", false), ("dil", "दिल", false), ("zindagi", "ज़िंदगी", false),
    ("duniya", "दुनिया", false), ("desh", "देश", false), ("bharat", "भारत", false),
    ("hindi", "हिंदी", false), ("ladka", "लड़का", false), ("ladki", "लड़की", false),
    ("log", "लोग", true), ("aadmi", "आदमी", false), ("bacche", "बच्चे", false),
    ("baccha", "बच्चा", false), ("maa", "माँ", false), ("papa", "पापा", true),
    ("didi", "दीदी", false), ("bhaiya", "भैया", false), ("ji", "जी", false),
    ("sawal", "सवाल", false), ("jawab", "जवाब", false), ("tarah", "तरह", false),
    ("tarike", "तरीके", false), ("tarika", "तरीका", false), ("zarurat", "ज़रूरत", false),
    ("madad", "मदद", false), ("dhyan", "ध्यान", false), ("shuru", "शुरू", false),
    ("khatam", "ख़त्म", false), ("khatm", "ख़त्म", false), ("wapas", "वापस", false),
    ("aage", "आगे", false), ("peeche", "पीछे", false), ("piche", "पीछे", false),
];

// ---------------------------------------------------------------------------
// Devanagari
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Sound {
    Consonant {
        ipa: &'static str,
        /// Written twice ("त्त"): held longer.
        doubled: bool,
        /// The unwritten "a" after this consonant was dropped.
        dropped_schwa: bool,
    },
    Vowel {
        ipa: &'static str,
        /// The unwritten "a" that follows a bare consonant.
        inherent: bool,
        nasal: bool,
    },
}

fn consonant(c: char) -> Option<&'static str> {
    Some(match c {
        'क' => "k", 'ख' => "kʰ", 'ग' => "ɡ", 'घ' => "ɡʰ", 'ङ' => "ŋ",
        'च' => "c", 'छ' => "cʰ", 'ज' => "ɟ", 'झ' => "ɟʰ", 'ञ' => "ɲ",
        'ट' => "ʈ", 'ठ' => "ʈʰ", 'ड' => "ɖ", 'ढ' => "ɖʰ", 'ण' => "ɳ",
        'त' => "t", 'थ' => "tʰ", 'द' => "d", 'ध' => "dʰ", 'न' => "n",
        'प' => "p", 'फ' => "pʰ", 'ब' => "b", 'भ' => "bʰ", 'म' => "m",
        'य' => "j", 'र' => "ɾ", 'ल' | 'ळ' => "l", 'व' => "ʋ",
        'श' => "ʃ", 'ष' => "ʂ", 'स' => "s", 'ह' => "h",
        // Letters with the dot below already attached.
        '\u{958}' => "q", '\u{959}' => "x", '\u{95A}' => "ɣ", '\u{95B}' => "z",
        '\u{95C}' => "r.", '\u{95D}' => "r.h", '\u{95E}' => "f", '\u{95F}' => "j",
        _ => return None,
    })
}

/// The sound of a consonant followed by the dot below (nukta).
fn dotted(ipa: &'static str) -> &'static str {
    match ipa {
        "k" => "q",
        "kʰ" => "x",
        "ɡ" => "ɣ",
        "ɟ" => "z",
        "ɖ" => "r.",
        "ɖʰ" => "r.h",
        "pʰ" => "f",
        other => other,
    }
}

/// Vowel letters and vowel signs. `None` for the vowel means "ri" (ऋ).
fn vowel(c: char) -> Option<&'static str> {
    Some(match c {
        'अ' => "ə",
        'आ' | 'ा' => "aː",
        'इ' | 'ि' => "ɪ",
        'ई' | 'ी' => "iː",
        'उ' | 'ु' => "ʊ",
        'ऊ' | 'ू' => "uː",
        'ए' | 'े' => "eː",
        'ऐ' | 'ै' => "ɛː",
        'ओ' | 'ो' => "oː",
        'औ' | 'ौ' => "ɔː",
        'ऑ' | 'ॉ' => "ɔ",
        'ऍ' | 'ॅ' => "ɛ",
        _ => return None,
    })
}

fn is_vowel_sign(c: char) -> bool {
    matches!(c, 'ा'..='ौ')
}

const VIRAMA: char = '्';
const NUKTA: char = '़';

/// Words said without stress.
const UNSTRESSED: &[&str] = &[
    "का", "की", "के", "को", "से", "है", "हैं", "था", "थी", "थे", "हो", "ये", "वो", "और", "हूँ",
    "क्या", "मैं", "में",
];

/// Pronounce one word written in Devanagari.
pub fn devanagari_word(word: &str) -> String {
    // "दे दो" style entries from the word list.
    if word.contains(' ') {
        return word.split(' ').map(devanagari_word).collect::<Vec<_>>().join(" ");
    }
    let mut sounds = letters_to_sounds(word);
    drop_schwas(&mut sounds);
    render(&sounds, !UNSTRESSED.contains(&word))
}

fn letters_to_sounds(word: &str) -> Vec<Sound> {
    let chars: Vec<char> = word.chars().collect();
    let mut sounds: Vec<Sound> = Vec::new();
    let plain = |ipa| Sound::Consonant { ipa, doubled: false, dropped_schwa: false };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(mut ipa) = consonant(c) {
            i += 1;
            if chars.get(i) == Some(&NUKTA) {
                ipa = dotted(ipa);
                i += 1;
            }
            // "ज्ञ" is said "gy".
            if ipa == "ɲ" && matches!(sounds.last(), Some(Sound::Consonant { ipa: "ɟ", .. })) {
                sounds.pop();
                sounds.push(plain("ɡ"));
                ipa = "j";
            }
            // "क्ष" is said "ksh", and "च्छ" breathes on both halves.
            if ipa == "ʂ" && matches!(sounds.last(), Some(Sound::Consonant { ipa: "k", .. })) {
                ipa = "ʃ";
            }
            if ipa == "cʰ" {
                if let Some(Sound::Consonant { ipa: first @ "c", .. }) = sounds.last_mut() {
                    *first = "cʰ";
                    sounds.push(plain("cʰ"));
                    ipa = "";
                }
            }
            // The same consonant twice is one long consonant.
            let repeated = matches!(sounds.last(), Some(Sound::Consonant { ipa: last, .. }) if *last == ipa);
            if ipa.is_empty() {
                // already added above
            } else if repeated {
                if let Some(Sound::Consonant { doubled, .. }) = sounds.last_mut() {
                    *doubled = true;
                }
            } else {
                // "क्य" and "प्य" hold the first consonant ("kya" -> "kkya").
                if ipa == "j" {
                    if let Some(Sound::Consonant { ipa: "k" | "p", doubled, .. }) = sounds.last_mut() {
                        *doubled = true;
                    }
                }
                sounds.push(plain(ipa));
            }
            match chars.get(i) {
                Some(&VIRAMA) => i += 1,
                Some(&'ृ') => {
                    sounds.push(plain("ɾ"));
                    sounds.push(Sound::Vowel { ipa: "ɪ", inherent: false, nasal: false });
                    i += 1;
                }
                Some(&sign) if is_vowel_sign(sign) => {
                    if let Some(ipa) = vowel(sign) {
                        sounds.push(Sound::Vowel { ipa, inherent: false, nasal: false });
                    }
                    i += 1;
                }
                _ => sounds.push(Sound::Vowel { ipa: "ə", inherent: true, nasal: false }),
            }
        } else if c == 'ऋ' {
            sounds.push(plain("ɾ"));
            sounds.push(Sound::Vowel { ipa: "ɪ", inherent: false, nasal: false });
            i += 1;
        } else if let Some(ipa) = vowel(c) {
            sounds.push(Sound::Vowel { ipa, inherent: false, nasal: false });
            i += 1;
        } else if matches!(c, 'ं' | 'ँ') {
            i += 1;
            let next = chars.get(i).copied().and_then(consonant);
            let after_schwa = matches!(sounds.last(), Some(Sound::Vowel { ipa: "ə", .. }));
            match (after_schwa, next) {
                // After a plain "a" the dot is a nasal consonant made in the
                // same place as the consonant that follows.
                (true, Some(next)) => sounds.push(plain(nasal_before(next))),
                (true, None) => sounds.push(plain("m")),
                // After any other vowel it makes the vowel nasal.
                _ => {
                    if let Some(Sound::Vowel { nasal, .. }) = sounds.last_mut() {
                        *nasal = true;
                    }
                }
            }
        } else if c == 'ः' {
            sounds.push(plain("h"));
            i += 1;
        } else {
            i += 1; // signs with no sound of their own
        }
    }
    sounds
}

fn nasal_before(next: &str) -> &'static str {
    match next.chars().next() {
        Some('k' | 'ɡ' | 'q' | 'x' | 'ɣ') => "ŋ",
        Some('c' | 'ɟ') => "ɲ",
        Some('ʈ' | 'ɖ') => "ɳ",
        Some('p' | 'b' | 'm' | 'f') => "m",
        _ => "n",
    }
}

fn is_vowel_sound(sound: Option<&Sound>) -> bool {
    matches!(sound, Some(Sound::Vowel { .. }))
}

fn is_single_consonant(sound: Option<&Sound>) -> bool {
    matches!(sound, Some(Sound::Consonant { doubled: false, .. }))
}

/// Hindi leaves out the unwritten "a" at the end of a word, and in the
/// middle wherever the consonants on both sides can lean on other vowels
/// ("करना" is "karna", not "karana").
fn drop_schwas(sounds: &mut Vec<Sound>) {
    let droppable = |s: &Sound| matches!(s, Sound::Vowel { inherent: true, nasal: false, .. });
    let mark_dropped = |sounds: &mut Vec<Sound>, at: usize| {
        if let Some(Sound::Consonant { dropped_schwa, .. }) = sounds.get_mut(at) {
            *dropped_schwa = true;
        }
    };

    let vowels = sounds.iter().filter(|s| is_vowel_sound(Some(s))).count();
    if vowels > 1 && sounds.last().is_some_and(droppable) {
        sounds.pop();
    }
    // Right to left: vowel, consonant, "a", consonant, vowel.
    let mut i = sounds.len();
    while i > 0 {
        i -= 1;
        if i < 2 || !droppable(&sounds[i]) {
            continue;
        }
        // The "a" after "h" stays ("udaaharan").
        let after_h = matches!(sounds.get(i - 1), Some(Sound::Consonant { ipa: "h", .. }));
        let fits = !after_h
            && is_vowel_sound(sounds.get(i - 2))
            && is_single_consonant(sounds.get(i - 1))
            && is_single_consonant(sounds.get(i + 1))
            && is_vowel_sound(sounds.get(i + 2));
        if fits {
            sounds.remove(i);
            mark_dropped(sounds, i);
        }
    }
}

/// Write the sounds out, with the stress mark and the vowel qualities that
/// depend on it.
fn render(sounds: &[Sound], stressed: bool) -> String {
    let vowel_positions: Vec<usize> = (0..sounds.len())
        .filter(|&i| is_vowel_sound(sounds.get(i)))
        .collect();
    let last_vowel = vowel_positions.last().copied();

    let is_long = |i: usize| match &sounds[i] {
        Sound::Vowel { ipa, nasal, .. } => *nasal || ipa.ends_with('ː'),
        Sound::Consonant { .. } => false,
    };
    // Consonants between this vowel and the next one (or the word end).
    let consonants_after = |i: usize| -> usize {
        sounds[i + 1..]
            .iter()
            .take_while(|s| !is_vowel_sound(Some(s)))
            .map(|s| if matches!(s, Sound::Consonant { doubled: true, .. }) { 2 } else { 1 })
            .sum()
    };
    let is_closed = |i: usize| {
        let after = consonants_after(i);
        if Some(i) == last_vowel { after >= 1 } else { after >= 2 }
    };

    // The stressed syllable: a long final syllable that ends in a consonant,
    // else the last long syllable before the final one, else the last closed
    // one before the final one, else the first.
    let primary = if !stressed {
        None
    } else {
        let earlier = &vowel_positions[..vowel_positions.len().saturating_sub(1)];
        match last_vowel {
            None => None,
            Some(last) if vowel_positions.len() == 1 => Some(last),
            Some(last) if is_long(last) && consonants_after(last) >= 1 => Some(last),
            Some(last) if consonants_after(last) >= 2 => Some(last),
            _ => earlier
                .iter()
                .rev()
                .find(|&&i| is_long(i))
                .or_else(|| earlier.iter().rev().find(|&&i| is_closed(i)))
                .or(vowel_positions.first())
                .copied(),
        }
    };
    // A final long vowel right after a dropped "a" gets a lighter stress
    // ("kar-NAA").
    let secondary = last_vowel.filter(|&last| {
        stressed
            && primary != Some(last)
            && last + 1 == sounds.len()
            && is_long(last)
            && matches!(sounds.get(last - 1), Some(Sound::Consonant { dropped_schwa: true, .. }))
    });

    let mut out = String::new();
    for (i, sound) in sounds.iter().enumerate() {
        match sound {
            Sound::Consonant { ipa, doubled, .. } => {
                out.push_str(ipa);
                if *doubled {
                    out.push('ː');
                }
            }
            Sound::Vowel { ipa, nasal, .. } => {
                if primary == Some(i) {
                    out.push('ˈ');
                } else if secondary == Some(i) {
                    out.push('ˌ');
                }
                let at_end = i + 1 == sounds.len();
                // A final "-ay" ("samay") has a more open vowel.
                let before_final_y = i + 2 == sounds.len()
                    && matches!(sounds.last(), Some(Sound::Consonant { ipa: "j", .. }));
                let ipa = match (*ipa, *nasal) {
                    ("ə", false) if before_final_y => "ɛ",
                    ("ə", false) if primary == Some(i) => "ʌ",
                    // Nasal vowels (the tilde is a separate character).
                    ("aː", true) => "a\u{303}",
                    ("iː" | "ɪ", true) => "i\u{303}",
                    ("uː" | "ʊ", true) => "u\u{303}",
                    ("eː", true) => "e\u{303}ː",
                    ("oː", true) => "o\u{303}",
                    ("ɛː" | "ɛ", true) => "ɛ\u{303}",
                    ("ɔː" | "ɔ", true) => "ɔ\u{303}",
                    ("ə", true) => "ʌ\u{303}",
                    // Long "i" and "u" are short at the end of a word.
                    ("iː", false) if at_end => "i",
                    ("uː", false) if at_end => "u",
                    (other, _) => other,
                };
                out.push_str(ipa);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Hindi in English letters
// ---------------------------------------------------------------------------

/// Sound out a romanised Hindi word that is not in the word list, following
/// the usual habits: "aa" is a long a, "ch" is च, "sh" is श, and so on.
pub fn roman_word(word: &str) -> String {
    let letters: Vec<char> = word
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let mut sounds: Vec<Sound> = Vec::new();
    let consonant = |ipa| Sound::Consonant { ipa, doubled: false, dropped_schwa: false };
    let vowel = |ipa| Sound::Vowel { ipa, inherent: false, nasal: false };

    let mut i = 0;
    while i < letters.len() {
        let rest = &letters[i..];
        let starts = |s: &str| rest.len() >= s.len() && rest.iter().zip(s.chars()).all(|(a, b)| *a == b);
        let at_end = |len: usize| i + len == letters.len();
        let (sound, len) = if starts("chh") {
            (consonant("cʰ"), 3)
        } else if starts("aa") {
            (vowel("aː"), 2)
        } else if starts("ai") || starts("ei") {
            (vowel("ɛː"), 2)
        } else if starts("au") || starts("ou") {
            (vowel("ɔː"), 2)
        } else if starts("ee") || starts("ii") {
            (vowel("iː"), 2)
        } else if starts("oo") || starts("uu") {
            (vowel("uː"), 2)
        } else if starts("ch") {
            (consonant("c"), 2)
        } else if starts("sh") {
            (consonant("ʃ"), 2)
        } else if starts("kh") {
            (consonant("kʰ"), 2)
        } else if starts("gh") {
            (consonant("ɡʰ"), 2)
        } else if starts("jh") {
            (consonant("ɟʰ"), 2)
        } else if starts("th") {
            (consonant("tʰ"), 2)
        } else if starts("dh") {
            (consonant("dʰ"), 2)
        } else if starts("ph") {
            (consonant("pʰ"), 2)
        } else if starts("bh") {
            (consonant("bʰ"), 2)
        } else {
            let sound = match rest[0] {
                // A final "a" is long ("accha"), elsewhere it is the short one.
                'a' if at_end(1) && !sounds.is_empty() => vowel("aː"),
                // "banao", "khae": the "a" before another vowel is long.
                'a' if matches!(rest.get(1), Some('o' | 'e')) => vowel("aː"),
                'a' => vowel("ə"),
                'i' if at_end(1) => vowel("iː"),
                'y' if at_end(1) && !sounds.is_empty() => vowel("iː"),
                'i' => vowel("ɪ"),
                'u' if at_end(1) => vowel("uː"),
                'u' => vowel("ʊ"),
                'e' => vowel("eː"),
                'o' => vowel("oː"),
                'k' | 'q' | 'c' => consonant("k"),
                'g' => consonant("ɡ"),
                'j' => consonant("ɟ"),
                't' => consonant("t"),
                'd' => consonant("d"),
                'n' => consonant("n"),
                'p' => consonant("p"),
                'b' => consonant("b"),
                'm' => consonant("m"),
                'y' => consonant("j"),
                'r' => consonant("ɾ"),
                'l' => consonant("l"),
                'v' | 'w' => consonant("ʋ"),
                's' => consonant("s"),
                'h' => consonant("h"),
                'z' => consonant("z"),
                'f' => consonant("f"),
                _ => consonant("ks"), // x
            };
            (sound, 1)
        };
        // A doubled letter is one long consonant.
        let same = matches!((&sound, sounds.last()),
            (Sound::Consonant { ipa: a, .. }, Some(Sound::Consonant { ipa: b, .. })) if a == b);
        if same {
            if let Some(Sound::Consonant { doubled, .. }) = sounds.last_mut() {
                *doubled = true;
            }
        } else {
            sounds.push(sound);
        }
        i += len;
    }
    render(&sounds, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devanagari_words() {
        for (word, expected) in [
            ("नमस्ते", "nəmˈʌsteː"),
            ("भारत", "bʰˈaːɾət"),
            ("घर", "ɡʰˈʌɾ"),
            ("करना", "kˈʌɾnˌaː"),
            ("कमल", "kˈʌməl"),
            ("कमला", "kˈʌmlˌaː"),
            ("किताब", "kɪtˈaːb"),
            ("सरकार", "səɾkˈaːɾ"),
            ("पानी", "pˈaːni"),
            ("है", "hɛː"),
            ("अंग", "ˈʌŋɡ"),
            ("चंचल", "cˈʌɲcəl"),
            ("पत्ता", "pˈʌtːaː"),
            ("क्या", "kːjaː"),
            ("ज़रूर", "zəɾˈuːɾ"),
            ("ज्ञान", "ɡjˈaːn"),
            ("कृपया", "kɾˈɪpjˌaː"),
            ("सड़क", "sˈʌr.ək"),
            ("अच्छा", "ˈʌcʰcʰaː"),
            ("क्षमा", "kʃˈʌmaː"),
            ("पसंद", "pəsˈʌnd"),
            ("उदाहरण", "ʊdˈaːhəɾəɳ"),
            ("समय", "sˈʌmɛj"),
        ] {
            assert_eq!(devanagari_word(word), expected, "{word}");
        }
    }

    /// Print the pronunciation of each word in a file, to compare against
    /// another pronouncer: LOUDR_WORDS=/path/words.txt cargo test ... -- --ignored
    #[test]
    #[ignore = "development aid"]
    fn print_pronunciations() {
        let path = std::env::var("LOUDR_WORDS").expect("set LOUDR_WORDS");
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let said: Vec<String> = line.split(' ').map(devanagari_word).collect();
            println!("{line}\t{}", said.join(" "));
        }
    }

    #[test]
    fn nasal_vowels_use_a_separate_tilde() {
        assert_eq!(devanagari_word("नहीं"), "nˈʌhi\u{303}");
        assert_eq!(devanagari_word("हिंदी"), "hˈi\u{303}di");
        assert_eq!(devanagari_word("में"), "me\u{303}ː");
    }

    #[test]
    fn hinglish_uses_the_word_list_and_keeps_english() {
        let said = phonemize("kal meeting hai, please join kar lena", |run| format!("<{run}>"));
        assert_eq!(said, "kˈʌl <meeting> hɛː<, please join> kˈʌɾ lˈeːnaː");
    }

    #[test]
    fn shared_words_are_hindi_only_next_to_hindi() {
        let english = phonemize("go to the main page", |run| format!("<{run}>"));
        assert_eq!(english, "<go to the main page>");
        let hindi = phonemize("main ghar ja raha hun", |run| format!("<{run}>"));
        assert!(!hindi.contains("<main"), "{hindi}");
    }

    #[test]
    fn mixed_scripts_and_the_hindi_full_stop() {
        let said = phonemize("यह app अच्छा है।", |run| format!("<{run}>"));
        assert!(said.contains("<app>") && said.ends_with('.'), "{said}");
    }

    #[test]
    fn unknown_roman_words_are_sounded_out() {
        assert_eq!(roman_word("chhutti"), "cʰˈʊtːi");
        assert_eq!(roman_word("khaana"), "kʰˈaːnaː");
    }
}
