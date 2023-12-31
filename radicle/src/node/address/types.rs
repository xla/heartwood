use std::cell::RefCell;
use std::hash;
use std::ops::{Deref, DerefMut};

use localtime::LocalTime;
use nonempty::NonEmpty;

use crate::collections::RandomMap;
use crate::node;
use crate::node::{Address, Alias};
use crate::prelude::Timestamp;

/// A map with the ability to randomly select values.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AddressBook<K: hash::Hash + Eq, V> {
    inner: RandomMap<K, V>,
    #[serde(skip)]
    rng: RefCell<fastrand::Rng>,
}

impl<K: hash::Hash + Eq, V> AddressBook<K, V> {
    /// Create a new address book.
    pub fn new(rng: fastrand::Rng) -> Self {
        Self {
            inner: RandomMap::with_hasher(rng.clone().into()),
            rng: RefCell::new(rng),
        }
    }

    /// Pick a random value in the book.
    pub fn sample(&self) -> Option<(&K, &V)> {
        self.sample_with(|_, _| true)
    }

    /// Pick a random value in the book matching a predicate.
    pub fn sample_with(&self, mut predicate: impl FnMut(&K, &V) -> bool) -> Option<(&K, &V)> {
        if let Some(pairs) = NonEmpty::from_vec(
            self.inner
                .iter()
                .filter(|(k, v)| predicate(*k, *v))
                .collect(),
        ) {
            let ix = self.rng.borrow_mut().usize(..pairs.len());
            let pair = pairs[ix]; // Can't fail.

            Some(pair)
        } else {
            None
        }
    }

    /// Return a new address book with the given RNG.
    pub fn with(self, rng: fastrand::Rng) -> Self {
        Self {
            inner: self.inner,
            rng: RefCell::new(rng),
        }
    }
}

impl<K: hash::Hash + Eq + Ord, V> AddressBook<K, V> {
    /// Return a shuffled iterator over the keys.
    pub fn shuffled(&self) -> std::vec::IntoIter<(&K, &V)> {
        let mut items = self.inner.iter().collect::<Vec<_>>();
        items.sort_by_key(|(k, _)| *k);
        self.rng.borrow_mut().shuffle(&mut items);

        items.into_iter()
    }

    /// Cycle through the keys at random. The random cycle repeats ad-infintum.
    pub fn cycle(&self) -> impl Iterator<Item = &K> {
        self.shuffled().map(|(k, _)| k).cycle()
    }
}

impl<K: hash::Hash + Eq, V> Deref for AddressBook<K, V> {
    type Target = RandomMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K: hash::Hash + Eq, V> DerefMut for AddressBook<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Node public data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Advertized alias.
    pub alias: Alias,
    /// Advertized features.
    pub features: node::Features,
    /// Advertized addresses
    pub addrs: Vec<KnownAddress>,
    /// Proof-of-work included in node announcement.
    pub pow: u32,
    /// When this data was published.
    pub timestamp: Timestamp,
}

/// A known address.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownAddress {
    /// Network address.
    pub addr: Address,
    /// Address of the peer who sent us this address.
    pub source: Source,
    /// Last time this address was used to successfully connect to a peer.
    pub last_success: Option<LocalTime>,
    /// Last time this address was tried.
    pub last_attempt: Option<LocalTime>,
}

impl KnownAddress {
    /// Create a new known address.
    pub fn new(addr: Address, source: Source) -> Self {
        Self {
            addr,
            source,
            last_success: None,
            last_attempt: None,
        }
    }
}

/// Address source. Specifies where an address originated from.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// An address that was shared by another peer.
    Peer,
    /// An bootstrap node address.
    Bootstrap,
    /// An address that came from some source external to the system, eg.
    /// specified by the user or added directly to the address manager.
    Imported,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Peer => write!(f, "Peer"),
            Self::Bootstrap => write!(f, "Bootstrap"),
            Self::Imported => write!(f, "Imported"),
        }
    }
}
