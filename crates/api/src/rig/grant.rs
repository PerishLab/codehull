use super::Seed;

pub(super) fn rows(
    seeds: &[Seed],
) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    seeds
        .iter()
        .map(|seed| (seed.who, seed.verb, seed.unit, seed.scope))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::SEEDS;
    use super::rows;

    #[test]
    fn order() {
        let held = rows(&SEEDS);
        assert_eq!(held.len(), SEEDS.len());
        for (seed, row) in SEEDS.iter().zip(&held) {
            assert_eq!(*row, (seed.who, seed.verb, seed.unit, seed.scope));
        }
        assert_eq!(held[0], ("anon", "see", "Actor", "all"));
    }
}
