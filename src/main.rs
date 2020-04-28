use rand::Rng;
use rand::seq::SliceRandom;

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::time::Instant;
use std::{fmt, mem, str};

pub const MAX_WORD_LEN: u8 = 30;

/// A data structure that compactly stores the word list.
struct WordList {
    word_data: Vec<Vec<u8>>,
    total_words: usize,
}

/// The index of a particular word in the list.
#[derive(Clone, Copy, Debug)]
struct Word {
    pub len: u8,
    pub idx: usize,
}

impl fmt::Display for WordList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "word list ({}):", self.total_words)?;
        for (i, data) in self.word_data.iter().enumerate() {
            let word_len = i + 1;
            writeln!(f, "- length {} ({}):", word_len, data.len() / word_len)?;
            for word in data.chunks_exact(word_len) {
                let word = str::from_utf8(word).unwrap();
                writeln!(f, "  - {}", word)?;
            }
        }
        Ok(())
    }
}

impl WordList {
    /// Create a new, empty word list.
    fn new() -> Self {
        Self {
            word_data: Vec::new(),
            total_words: 0,
        }
    }
    
    /// Count the number of words with a given length.
    fn count_with_length(&self, len: u8) -> usize {
        let len = len as usize;
        self.word_data[len - 1].len() / len
    }

    /// Add a word to the list and return its index.
    fn insert(&mut self, word: &str) -> Word {
        let len = word.len();
        assert!(0 < len && len < (MAX_WORD_LEN as usize));
        while self.word_data.len() < len {
            self.word_data.push(Vec::new());
        }
        let data_vec = &mut self.word_data[len - 1];
        let idx = data_vec.len() / len;
        for b in word.bytes() {
            assert!(b.is_ascii_lowercase());
            data_vec.push(b);
        }
        self.total_words += 1;
        Word {
            len: len as u8,
            idx,
        }
    }

    /// Get a slice representing the letters of the word.
    fn get(&self, word: Word) -> &[u8] {
        let Word { len, idx } = word;
        let len = len as usize;
        &self.word_data[len - 1][len * idx .. len * (idx + 1)]
    }

    /// Return the index of a random word.
    fn random<R: Rng>(&self, rng: &mut R) -> Word {
        let idx = rng.gen_range(0, self.total_words);
        let mut words_so_far = 0;
        for (i, data_vec) in self.word_data.iter().enumerate() {
            let len = (i + 1) as u8;
            let words = data_vec.len() / (len as usize);
            words_so_far += words;
            if words_so_far > idx {
                return Word { len, idx: idx - (words_so_far - words) }
            }
        }
        unreachable!()
    }
    
    /// Return an iterator over all words.
    fn iter(&self) -> impl Iterator<Item=Word> {
        let data_vec_lens: Vec<usize> =
            self.word_data.iter()
                .enumerate()
                .map(|(i, data_vec)| data_vec.len() / (i + 1))
                .collect();
        let mut i = 0;
        let mut j = 0;
        std::iter::from_fn(move || {
            while !(j < *data_vec_lens.get(i)?) {
                i += 1;
                j = 0;
            }
            let len = (i + 1) as u8;
            let idx = j;
            j += 1;
            Some(Word { len, idx })
        })
    }
}

/// Represents an executioner that can be queried about the word it has chosen.
trait Executioner: Sized {
    /// Start the game with a given word.
    fn init(word: Word, words: &WordList) -> Self;
    /// Start the game with a random choice of word.
    fn choose<R: Rng>(words: &WordList, rng: &mut R) -> Self where R: Rng {
        let word = words.random(rng);
        Self::init(word, words)
    }
    /// Given a letter, push to `idxs` the indices of all occurences of that letter
    /// within the chosen word.
    fn guess(&mut self, words: &WordList, letter: u8, idxs: &mut Vec<u8>);
    /// Return the length of the chosen word.
    fn word_len(&self) -> u8;
    /// Return the number of times `guess` was called but did not modify `idxs`.
    fn wrong_guesses(&self) -> usize;
}

/// An executioner that picks a word in advance and describes it honestly.
struct HonestExecutioner {
    word: Word,
    wrong_guesses: usize,
}

impl Executioner for HonestExecutioner {
    fn init(word: Word, _words: &WordList) -> Self {
        Self { word, wrong_guesses: 0 }
    }
    fn guess(&mut self, words: &WordList, letter: u8, idxs: &mut Vec<u8>) {
        idxs.clear();
        let word = words.get(self.word);
        for (i, &c) in word.iter().enumerate() {
            if c == letter {
                idxs.push(i as u8);
            }
        }
        if idxs.is_empty() {
            self.wrong_guesses += 1;
        }
    }
    fn word_len(&self) -> u8 { self.word.len }
    fn wrong_guesses(&self) -> usize { self.wrong_guesses }
}

/// Represents a particular way to play the game.
trait Strategy {
    fn play<E: Executioner, R: Rng>(
        &mut self,
        executioner: &mut E,
        words: &WordList,
        rng: &mut R,
    );
}

/// A strategy that guesses letters in a random order.
struct RandomStrategy {
    guesses: Vec<u8>,
    idxs_buf: Vec<u8>,
}

impl RandomStrategy {
    fn new() -> Self {
        Self {
            guesses: Vec::with_capacity(26),
            idxs_buf: Vec::new(),
        }
    }
}

impl Strategy for RandomStrategy {
    fn play<E: Executioner, R: Rng>(
        &mut self,
        executioner: &mut E,
        words: &WordList,
        rng: &mut R,
    ) {
        let word_len = executioner.word_len();
        self.guesses.clear();
        self.guesses.extend(b'a'..=b'z');
        self.guesses.shuffle(rng);
        let mut guessed_letters = 0;
        while guessed_letters < word_len {
            let letter = self.guesses.pop().unwrap();
            executioner.guess(words, letter, &mut self.idxs_buf);
            let occurences = self.idxs_buf.len() as u8;
            guessed_letters += occurences;
        }
        assert_eq!(guessed_letters, word_len);
    }
}

/// A strategy that guesses letters in order of their frequency.
struct SimpleStrategy {
    idxs_buf: Vec<u8>,
}

impl SimpleStrategy {
    fn new() -> Self { Self { idxs_buf: Vec::new() } }
}

/// Computed from `/usr/share/dict/words`. The version
/// given in the video is similar: "eianorstlcudpmhgybfvkwzxqj".
static OPTIMAL_ORDER: [u8; 26] = *b"eiaorntslcupmdhygbfvkwzxqj";

impl Strategy for SimpleStrategy {
    fn play<E: Executioner, R: Rng>(
        &mut self,
        executioner: &mut E,
        words: &WordList,
        _rng: &mut R,
    ) {
        let word_len = executioner.word_len();
        let mut guessed_letters = 0;
        let mut i = 0;
        while guessed_letters < word_len {
            let letter = OPTIMAL_ORDER[i];
            executioner.guess(words, letter, &mut self.idxs_buf);
            let occurences = self.idxs_buf.len() as u8;
            guessed_letters += occurences;
            i += 1;
        }
        assert_eq!(guessed_letters, word_len);
    }
}

/// A strategy that uses the information returned by the executioner
/// to guess letters in the conjectured optimal order.
///
/// Specifically, it guesses whichever letter appears most often in
/// the set of remaining possible words.
struct EpicStrategy {
    candidates: HashSet<usize>,
    remaining_letters: Vec<u8>,
    idxs_buf: Vec<u8>,
}

impl EpicStrategy {
    fn new() -> Self {
        Self {
            candidates: HashSet::new(),
            remaining_letters: Vec::new(),
            idxs_buf: Vec::new(),
        }
    }
}

impl Strategy for EpicStrategy {
    fn play<E: Executioner, R: Rng>(
        &mut self,
        executioner: &mut E,
        words: &WordList,
        _rng: &mut R,
    ) {
        self.remaining_letters.clear();
        self.remaining_letters.extend(b'a'..=b'z');

        // Start by considering all possible words with the specified length.
        let word_len = executioner.word_len();
        self.candidates.clear();
        self.candidates.extend(0..words.count_with_length(word_len));
        
        while self.candidates.len() > 1 {
            // Identify the frequencies with which each letter appears in the candidate words.
            let mut letter_frequencies: [usize; 26] = [0; 26];
            for &word in self.candidates.iter() {
                // Make sure not to double-count letters.
                let mut letter_appearances: [bool; 26] = [false; 26];
                for letter in words.get(Word { len: word_len, idx: word }) {
                    let idx = (letter - b'a') as usize;
                    if !mem::replace(&mut letter_appearances[idx], true) {
                        letter_frequencies[idx] += 1;
                    }
                }
            }

            // Identify the letter that hasn't been guessed yet but which appears in the most
            // words.
            let (i, guess) = self.remaining_letters
                .iter().copied().enumerate()
                .max_by_key(|(_, c)| letter_frequencies[(c - b'a') as usize])
                .unwrap();
            self.remaining_letters.swap_remove(i);
            executioner.guess(words, guess, &mut self.idxs_buf);

            // Keep only the candidates that have that letter in only the specified positions.
            let idxs_buf = &mut self.idxs_buf;
            self.candidates.retain(|&word| {
                let word = Word { len: word_len, idx: word };
                words.get(word).iter().enumerate().all(|(i, &letter)| {
                    let does_match   = letter == guess;
                    let should_match = idxs_buf.contains(&(i as u8));
                    does_match == should_match
                })
            });
        }

        assert_eq!(self.candidates.len(), 1);
    }
}

fn describe_strategy<S, R>(desc: &str, strategy: &mut S, words: &WordList, rng: &mut R)
    where
        S: Strategy,
        R: Rng
{
    let start = Instant::now();
    println!("Strategy '{}':", desc);

    let mut scores = Vec::<(Word, usize)>::new();

    let mut total_wrong_guesses = 0;
    let mut total_words = 0;
    for word in words.iter() {
        let mut exec = HonestExecutioner::init(word, words);
        strategy.play(&mut exec, words, rng);
        scores.push((word, exec.wrong_guesses()));
        total_wrong_guesses += exec.wrong_guesses();
        total_words += 1;
    }

    println!("  Average # of wrong guesses: {}",
             (total_wrong_guesses as f64) / (total_words as f64));

    scores.sort_by_key(|(_, score)| std::cmp::Reverse(*score));

    println!("  Sorted words by guessability.");

    let path = format!("{}.txt", desc);
    let mut file = std::fs::File::create(&path).unwrap();
    for (word, wrong_guesses) in scores {
        writeln!(
            &mut file,
            "{} {}",
            wrong_guesses,
            str::from_utf8(words.get(word)).unwrap(),
        ).unwrap();
    }

    println!("  Wrote report to file '{}'.", path);

    println!("  Completed in {:.3?}.", start.elapsed());
}

fn main() {
    let stdin = io::stdin();
    let stdin = stdin.lock();

    let mut words = WordList::new();

    for line in stdin.lines() {
        let line = line.unwrap();
        words.insert(&line);
    }

    let mut rng = rand::thread_rng();

    describe_strategy("random", &mut RandomStrategy::new(), &words, &mut rng);
    describe_strategy("simple", &mut SimpleStrategy::new(), &words, &mut rng);
    describe_strategy("epic", &mut EpicStrategy::new(), &words, &mut rng);
}
