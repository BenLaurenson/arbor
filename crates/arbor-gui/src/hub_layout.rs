use serde::{Deserialize, Serialize};

/// Direction of a split in the hub layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) enum SplitDirection {
    /// Side by side (left | right).
    Horizontal,
    /// Stacked (top / bottom).
    Vertical,
}

/// Where to place a new terminal relative to an existing pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum DropZone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

/// A node in the binary split tree that defines the hub layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) enum HubPane {
    Split {
        direction: SplitDirection,
        /// Position of the divider, 0.0–1.0.
        ratio: f32,
        first: Box<HubPane>,
        second: Box<HubPane>,
    },
    /// A leaf pane containing a terminal session.
    Terminal(u64),
    /// An empty placeholder pane.
    Empty,
}

impl Eq for HubPane {}

#[allow(dead_code)]
impl HubPane {
    /// Returns `true` if `terminal_id` exists anywhere in this tree.
    pub(crate) fn contains_terminal(&self, terminal_id: u64) -> bool {
        match self {
            Self::Terminal(id) => *id == terminal_id,
            Self::Split { first, second, .. } => {
                first.contains_terminal(terminal_id) || second.contains_terminal(terminal_id)
            },
            Self::Empty => false,
        }
    }

    /// Collect all terminal IDs in this tree.
    pub(crate) fn terminal_ids(&self) -> Vec<u64> {
        match self {
            Self::Terminal(id) => vec![*id],
            Self::Split { first, second, .. } => {
                let mut ids = first.terminal_ids();
                ids.extend(second.terminal_ids());
                ids
            },
            Self::Empty => Vec::new(),
        }
    }

    /// Returns the number of terminals in the tree.
    pub(crate) fn terminal_count(&self) -> usize {
        match self {
            Self::Terminal(_) => 1,
            Self::Split { first, second, .. } => first.terminal_count() + second.terminal_count(),
            Self::Empty => 0,
        }
    }

    /// Returns `true` if this pane is `Empty`.
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Split the pane containing `target_id` in the given direction,
    /// placing `new_id` in the position indicated by `zone`.
    /// Returns `true` if the split was performed.
    pub(crate) fn split_at(&mut self, target_id: u64, new_id: u64, zone: DropZone) -> bool {
        match self {
            Self::Terminal(id) if *id == target_id => {
                let existing = Box::new(Self::Terminal(target_id));
                let new_pane = Box::new(Self::Terminal(new_id));
                match zone {
                    DropZone::Center => {
                        // Replace the terminal
                        *self = Self::Terminal(new_id);
                    },
                    DropZone::Left => {
                        *self = Self::Split {
                            direction: SplitDirection::Horizontal,
                            ratio: 0.5,
                            first: new_pane,
                            second: existing,
                        };
                    },
                    DropZone::Right => {
                        *self = Self::Split {
                            direction: SplitDirection::Horizontal,
                            ratio: 0.5,
                            first: existing,
                            second: new_pane,
                        };
                    },
                    DropZone::Top => {
                        *self = Self::Split {
                            direction: SplitDirection::Vertical,
                            ratio: 0.5,
                            first: new_pane,
                            second: existing,
                        };
                    },
                    DropZone::Bottom => {
                        *self = Self::Split {
                            direction: SplitDirection::Vertical,
                            ratio: 0.5,
                            first: existing,
                            second: new_pane,
                        };
                    },
                }
                true
            },
            Self::Split { first, second, .. } => {
                first.split_at(target_id, new_id, zone) || second.split_at(target_id, new_id, zone)
            },
            _ => false,
        }
    }

    /// Split the pane containing `target_id`, placing an `Empty` pane
    /// in the position indicated by `zone`. Unlike `split_at`, this does
    /// not require a new terminal ID — the empty slot can be filled later.
    pub(crate) fn split_with_empty(&mut self, target_id: u64, zone: DropZone) -> bool {
        match self {
            Self::Terminal(id) if *id == target_id => {
                let existing = Box::new(Self::Terminal(target_id));
                let empty = Box::new(Self::Empty);
                match zone {
                    DropZone::Center => false, // replacing with empty makes no sense
                    DropZone::Left => {
                        *self = Self::Split {
                            direction: SplitDirection::Horizontal,
                            ratio: 0.5,
                            first: empty,
                            second: existing,
                        };
                        true
                    },
                    DropZone::Right => {
                        *self = Self::Split {
                            direction: SplitDirection::Horizontal,
                            ratio: 0.5,
                            first: existing,
                            second: empty,
                        };
                        true
                    },
                    DropZone::Top => {
                        *self = Self::Split {
                            direction: SplitDirection::Vertical,
                            ratio: 0.5,
                            first: empty,
                            second: existing,
                        };
                        true
                    },
                    DropZone::Bottom => {
                        *self = Self::Split {
                            direction: SplitDirection::Vertical,
                            ratio: 0.5,
                            first: existing,
                            second: empty,
                        };
                        true
                    },
                }
            },
            Self::Split { first, second, .. } => {
                first.split_with_empty(target_id, zone) || second.split_with_empty(target_id, zone)
            },
            _ => false,
        }
    }

    /// Remove a terminal from the tree and collapse any resulting
    /// single-child splits. Returns `true` if the terminal was found
    /// and removed.
    pub(crate) fn remove_terminal(&mut self, terminal_id: u64) -> bool {
        match self {
            Self::Terminal(id) if *id == terminal_id => {
                *self = Self::Empty;
                true
            },
            Self::Split { first, second, .. } => {
                let removed =
                    first.remove_terminal(terminal_id) || second.remove_terminal(terminal_id);
                if removed {
                    self.collapse_if_needed();
                }
                removed
            },
            _ => false,
        }
    }

    /// If one child is empty after a removal, collapse the split to
    /// the remaining child.
    fn collapse_if_needed(&mut self) {
        if let Self::Split { first, second, .. } = self {
            if first.is_empty() {
                *self = *second.clone();
            } else if second.is_empty() {
                *self = *first.clone();
            }
        }
    }

    /// Add a terminal to the tree. If the tree is empty, become a
    /// `Terminal` leaf. If it's a single terminal, split horizontally.
    /// If it's a split, add to the second (right/bottom) pane
    /// recursively until an empty slot is found.
    pub(crate) fn add_terminal(&mut self, terminal_id: u64) {
        if self.contains_terminal(terminal_id) {
            return;
        }
        match self {
            Self::Empty => {
                *self = Self::Terminal(terminal_id);
            },
            Self::Terminal(existing_id) => {
                let existing = *existing_id;
                *self = Self::Split {
                    direction: SplitDirection::Horizontal,
                    ratio: 0.5,
                    first: Box::new(Self::Terminal(existing)),
                    second: Box::new(Self::Terminal(terminal_id)),
                };
            },
            Self::Split { first, second, .. } => {
                // Prefer filling empty slots, then add to the smaller side
                if first.is_empty() {
                    **first = Self::Terminal(terminal_id);
                } else if second.is_empty() {
                    **second = Self::Terminal(terminal_id);
                } else if first.terminal_count() <= second.terminal_count() {
                    first.add_terminal(terminal_id);
                } else {
                    second.add_terminal(terminal_id);
                }
            },
        }
    }

    /// Update a split ratio at the split containing the given path.
    /// `path` is a sequence of 0 (first) or 1 (second) indices
    /// navigating the tree. The ratio is clamped to [0.15, 0.85].
    pub(crate) fn set_ratio_at_path(&mut self, path: &[usize], ratio: f32) {
        if path.is_empty() {
            if let Self::Split {
                ratio: current_ratio,
                ..
            } = self
            {
                *current_ratio = ratio.clamp(0.15, 0.85);
            }
            return;
        }
        if let Self::Split { first, second, .. } = self {
            match path[0] {
                0 => first.set_ratio_at_path(&path[1..], ratio),
                1 => second.set_ratio_at_path(&path[1..], ratio),
                _ => {},
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_pane_has_no_terminals() {
        let pane = HubPane::Empty;
        assert!(pane.is_empty());
        assert_eq!(pane.terminal_count(), 0);
        assert!(pane.terminal_ids().is_empty());
    }

    #[test]
    fn single_terminal_pane() {
        let pane = HubPane::Terminal(42);
        assert!(!pane.is_empty());
        assert_eq!(pane.terminal_count(), 1);
        assert!(pane.contains_terminal(42));
        assert!(!pane.contains_terminal(99));
        assert_eq!(pane.terminal_ids(), vec![42]);
    }

    #[test]
    fn add_terminal_to_empty() {
        let mut pane = HubPane::Empty;
        pane.add_terminal(1);
        assert_eq!(pane, HubPane::Terminal(1));
    }

    #[test]
    fn add_terminal_to_single_creates_horizontal_split() {
        let mut pane = HubPane::Terminal(1);
        pane.add_terminal(2);
        assert_eq!(pane.terminal_count(), 2);
        assert!(pane.contains_terminal(1));
        assert!(pane.contains_terminal(2));
        match &pane {
            HubPane::Split { direction, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
            },
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn add_duplicate_terminal_is_noop() {
        let mut pane = HubPane::Terminal(1);
        pane.add_terminal(1);
        assert_eq!(pane, HubPane::Terminal(1));
    }

    #[test]
    fn remove_terminal_from_leaf() {
        let mut pane = HubPane::Terminal(1);
        assert!(pane.remove_terminal(1));
        assert!(pane.is_empty());
    }

    #[test]
    fn remove_terminal_collapses_split() {
        let mut pane = HubPane::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(HubPane::Terminal(1)),
            second: Box::new(HubPane::Terminal(2)),
        };
        assert!(pane.remove_terminal(1));
        assert_eq!(pane, HubPane::Terminal(2));
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut pane = HubPane::Terminal(1);
        assert!(!pane.remove_terminal(99));
        assert_eq!(pane, HubPane::Terminal(1));
    }

    #[test]
    fn split_at_creates_correct_layout() {
        let mut pane = HubPane::Terminal(1);
        assert!(pane.split_at(1, 2, DropZone::Right));

        match &pane {
            HubPane::Split {
                direction,
                first,
                second,
                ..
            } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(**first, HubPane::Terminal(1));
                assert_eq!(**second, HubPane::Terminal(2));
            },
            _ => panic!("expected horizontal split"),
        }
    }

    #[test]
    fn split_at_left_puts_new_first() {
        let mut pane = HubPane::Terminal(1);
        assert!(pane.split_at(1, 2, DropZone::Left));

        match &pane {
            HubPane::Split { first, second, .. } => {
                assert_eq!(**first, HubPane::Terminal(2));
                assert_eq!(**second, HubPane::Terminal(1));
            },
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn split_at_bottom_creates_vertical_split() {
        let mut pane = HubPane::Terminal(1);
        assert!(pane.split_at(1, 2, DropZone::Bottom));

        match &pane {
            HubPane::Split {
                direction,
                first,
                second,
                ..
            } => {
                assert_eq!(*direction, SplitDirection::Vertical);
                assert_eq!(**first, HubPane::Terminal(1));
                assert_eq!(**second, HubPane::Terminal(2));
            },
            _ => panic!("expected vertical split"),
        }
    }

    #[test]
    fn split_at_center_replaces_terminal() {
        let mut pane = HubPane::Terminal(1);
        assert!(pane.split_at(1, 2, DropZone::Center));
        assert_eq!(pane, HubPane::Terminal(2));
    }

    #[test]
    fn set_ratio_at_root() {
        let mut pane = HubPane::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(HubPane::Terminal(1)),
            second: Box::new(HubPane::Terminal(2)),
        };
        pane.set_ratio_at_path(&[], 0.3);
        match &pane {
            HubPane::Split { ratio, .. } => assert!((ratio - 0.3).abs() < f32::EPSILON),
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn set_ratio_clamps_to_bounds() {
        let mut pane = HubPane::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(HubPane::Terminal(1)),
            second: Box::new(HubPane::Terminal(2)),
        };
        pane.set_ratio_at_path(&[], 0.01);
        match &pane {
            HubPane::Split { ratio, .. } => assert!((ratio - 0.15).abs() < f32::EPSILON),
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn complex_tree_operations() {
        let mut pane = HubPane::Empty;
        pane.add_terminal(1);
        pane.add_terminal(2);
        pane.add_terminal(3);

        assert_eq!(pane.terminal_count(), 3);
        assert!(pane.contains_terminal(1));
        assert!(pane.contains_terminal(2));
        assert!(pane.contains_terminal(3));

        // Remove from the middle
        assert!(pane.remove_terminal(2));
        assert_eq!(pane.terminal_count(), 2);
        assert!(!pane.contains_terminal(2));

        // The remaining terminals should still be accessible
        let ids = pane.terminal_ids();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
    }

    #[test]
    fn add_terminal_fills_empty_slots_first() {
        let mut pane = HubPane::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(HubPane::Empty),
            second: Box::new(HubPane::Terminal(1)),
        };
        pane.add_terminal(2);
        match &pane {
            HubPane::Split { first, .. } => {
                assert_eq!(**first, HubPane::Terminal(2));
            },
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn serialization_round_trips() {
        let pane = HubPane::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(HubPane::Terminal(1)),
            second: Box::new(HubPane::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.3,
                first: Box::new(HubPane::Terminal(2)),
                second: Box::new(HubPane::Terminal(3)),
            }),
        };
        let json = serde_json::to_string(&pane).unwrap();
        let deserialized: HubPane = serde_json::from_str(&json).unwrap();
        assert_eq!(pane, deserialized);
    }
}
