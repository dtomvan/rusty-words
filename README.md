# rusty-words

Learn your words in your terminal.

## What it does now
- Manage your words lists
- Import them from TSV
- Practice by writing
- Put them in folders
- Create new ones from scratch (TSV)

## What it will do in the near future
- Practice by multiple choice
- Export to TSV
- Configure how you should be judged (how many correct answers given before
  accepting a term as learned, resetting your progress on a term when you got it
  wrong, how many words to keep in rotation, how to check if the user is correct etc.)
- Display itself as a GUI

## What it may be able to do at some point
- Keep track of a "score" per term, lowering it exponentially each day you
  haven't practiced it. That way, prioritize words you haven't learned in a
  while.

## Project goal
My goal is to:
- Practice writing a TUI in rust
- Make something you could (but not nessecarily want to) use for practicing
  term-definition pairs.

The goal is NOT:
- Make a complete drop-in replacement for some pre-existing software, even
  though I've also made `word-tools`, a helper for converting between TSV and
  T2K, Teach2000's file format.
