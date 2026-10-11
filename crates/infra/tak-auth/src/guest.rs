use std::sync::LazyLock;

use rand::seq::IndexedRandom;

const ADJECTIVES_TEXT: &str = include_str!("adjectives.txt");

static ADJECTIVES: LazyLock<Vec<&str>> = LazyLock::new(|| {
    ADJECTIVES_TEXT
        .lines()
        .flat_map(|line| line.split(","))
        .map(|s| s.trim())
        .filter(|line| !line.is_empty())
        .collect()
});

const NOUNS_TEXT: &str = include_str!("nouns.txt");

static NOUNS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    NOUNS_TEXT
        .lines()
        .flat_map(|line| line.split(","))
        .map(|s| s.trim())
        .filter(|line| !line.is_empty())
        .collect()
});

pub fn generate_guest_name() -> String {
    let mut rng = rand::rng();
    format!(
        "{} {}",
        ADJECTIVES.choose(&mut rng).unwrap(),
        NOUNS.choose(&mut rng).unwrap()
    )
}
