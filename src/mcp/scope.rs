use clap::ValueEnum;

/// A capability group that gates which tools get registered.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Read,
    Write,
    Delete,
}

/// Bit set of [`Scope`]s, so scope checks are a mask test instead of a `Vec` scan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScopeSet(u8);

impl ScopeSet {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const DELETE: Self = Self(1 << 2);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl From<Scope> for ScopeSet {
    fn from(scope: Scope) -> Self {
        match scope {
            Scope::Read => Self::READ,
            Scope::Write => Self::WRITE,
            Scope::Delete => Self::DELETE,
        }
    }
}

impl FromIterator<Scope> for ScopeSet {
    fn from_iter<I: IntoIterator<Item = Scope>>(iter: I) -> Self {
        Self(iter.into_iter().fold(0, |bits, s| bits | Self::from(s).0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_contains_nothing() {
        let set = ScopeSet::default();
        assert!(!set.contains(ScopeSet::READ));
        assert!(!set.contains(ScopeSet::WRITE));
        assert!(!set.contains(ScopeSet::DELETE));
    }

    #[test]
    fn single_scope_does_not_imply_others() {
        let set = ScopeSet::from_iter([Scope::Read]);
        assert!(set.contains(ScopeSet::READ));
        assert!(!set.contains(ScopeSet::WRITE));
        assert!(!set.contains(ScopeSet::DELETE));
    }

    #[test]
    fn collects_every_scope() {
        let set = ScopeSet::from_iter([Scope::Read, Scope::Write, Scope::Delete]);
        assert!(set.contains(ScopeSet::READ));
        assert!(set.contains(ScopeSet::WRITE));
        assert!(set.contains(ScopeSet::DELETE));
    }

    #[test]
    fn repeating_a_scope_is_a_no_op() {
        let once = ScopeSet::from_iter([Scope::Write]);
        let twice = ScopeSet::from_iter([Scope::Write, Scope::Write]);
        assert_eq!(once, twice);
    }
}
