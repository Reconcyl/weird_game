# Weird Game

This is a Rust implementation of some of the concepts featured in the video [*hangman is a weird game*](https://www.youtube.com/watch?v=le5uGqHKll8) by jan Misali. The intent is to measure the performance of various strategies for playing the game [Hangman](https://en.wikipedia.org/wiki/Hangman_%28game%29).

In its current form, this program takes in a list of lowercase ASCII words line-by-line from STDIN and tests three different strategies against these words. These strategies are:

- Guessing letters in a random order.
- Guessing letters in order of their frequency (by my measurement).
- Guessing letters using the proposed optimal strategy (guess whichever letter appears in the greatest number of candidate words based on available information).

The dictionary I used for testing was generated using the following command:

    cat /usr/share/dict/words | grep -x '[a-z]\+'

On my machine, this had 210,687 entries and produced the following results:

- On average, the first strategy made 16.06 incorrect guesses.
- On average, the second strategy made 9.62 incorrect guesses. The hardest words to guess included `juju`, `ajaja`, and `jab`.
- On average, the third strategy made 1.70 incorrect guesses (further testing revealed that this was heavily dependent on the size of the provided dictionary). The hardest words to guess included `vuln`, `cack`, `hill`, and `scuff`.

Testing the third strategy on the entire word list required under fifteen minutes, averaging out to about 300 words per second.
