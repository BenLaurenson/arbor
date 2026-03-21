use serde::{Deserialize, Serialize};

/// Maximum terminals per page (2 rows × 3 columns).
pub(crate) const MAX_PER_PAGE: usize = 6;

/// How the hub grid arranges terminal panes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum HubLayoutMode {
    /// Auto-fit: 1=full, 2=1×2, 3=1×3, 4=2×2, 5-6=2×3
    #[default]
    Auto,
    /// Group terminals by project with horizontal swim lanes.
    SwimLanes,
}

/// Flat grid layout for the Agent Hub.
/// Replaces the binary split tree with a simple ordered list of terminals
/// rendered in an equal-sized grid with pagination.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct HubGrid {
    /// All terminal IDs in display order.
    pub(crate) terminals: Vec<u64>,
    /// Current visible page (0-indexed).
    pub(crate) current_page: usize,
    /// Layout mode.
    pub(crate) layout: HubLayoutMode,
}

#[allow(dead_code)]
impl HubGrid {
    pub(crate) fn is_empty(&self) -> bool {
        self.terminals.is_empty()
    }

    pub(crate) fn terminal_count(&self) -> usize {
        self.terminals.len()
    }

    pub(crate) fn contains_terminal(&self, terminal_id: u64) -> bool {
        self.terminals.contains(&terminal_id)
    }

    #[allow(dead_code)]
    pub(crate) fn terminal_ids(&self) -> &[u64] {
        &self.terminals
    }

    /// Add a terminal to the grid. If already present, no-op.
    /// Auto-navigates to the page containing the new terminal.
    pub(crate) fn add_terminal(&mut self, terminal_id: u64) {
        if self.terminals.contains(&terminal_id) {
            return;
        }
        self.terminals.push(terminal_id);
        // Auto-navigate to the page containing the new terminal
        self.current_page = self.page_for_terminal(terminal_id).unwrap_or(0);
    }

    /// Remove a terminal from the grid.
    pub(crate) fn remove_terminal(&mut self, terminal_id: u64) {
        self.terminals.retain(|id| *id != terminal_id);
        // Clamp current page
        let max_page = self.page_count().saturating_sub(1);
        if self.current_page > max_page {
            self.current_page = max_page;
        }
    }

    /// Swap two terminals' positions in the grid.
    pub(crate) fn swap_terminals(&mut self, a: u64, b: u64) {
        let pos_a = self.terminals.iter().position(|id| *id == a);
        let pos_b = self.terminals.iter().position(|id| *id == b);
        if let (Some(i), Some(j)) = (pos_a, pos_b) {
            self.terminals.swap(i, j);
        }
    }

    /// Total number of pages.
    pub(crate) fn page_count(&self) -> usize {
        if self.terminals.is_empty() {
            1
        } else {
            self.terminals.len().div_ceil(MAX_PER_PAGE)
        }
    }

    /// Which page a terminal is on.
    pub(crate) fn page_for_terminal(&self, terminal_id: u64) -> Option<usize> {
        self.terminals
            .iter()
            .position(|id| *id == terminal_id)
            .map(|idx| idx / MAX_PER_PAGE)
    }

    /// Terminal IDs for the current page.
    pub(crate) fn page_terminals(&self) -> &[u64] {
        let start = self.current_page * MAX_PER_PAGE;
        let end = (start + MAX_PER_PAGE).min(self.terminals.len());
        if start >= self.terminals.len() {
            &[]
        } else {
            &self.terminals[start..end]
        }
    }

    /// Navigate to next page (wraps around).
    pub(crate) fn next_page(&mut self) {
        let count = self.page_count();
        if count > 1 {
            self.current_page = (self.current_page + 1) % count;
        }
    }

    /// Navigate to previous page (wraps around).
    pub(crate) fn prev_page(&mut self) {
        let count = self.page_count();
        if count > 1 {
            self.current_page = (self.current_page + count - 1) % count;
        }
    }

    /// Go to a specific page.
    pub(crate) fn go_to_page(&mut self, page: usize) {
        if page < self.page_count() {
            self.current_page = page;
        }
    }
}

/// Calculate grid dimensions (rows, cols) for a given terminal count.
/// Max: 2 rows × 3 columns.
pub(crate) fn grid_dimensions(count: usize) -> (usize, usize) {
    match count {
        0 => (0, 0),
        1 => (1, 1),
        2 => (1, 2),
        3 => (1, 3),
        4 => (2, 2),
        5 | 6 => (2, 3),
        _ => (2, 3), // clamped to max
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_grid() {
        let grid = HubGrid::default();
        assert!(grid.is_empty());
        assert_eq!(grid.terminal_count(), 0);
        assert_eq!(grid.page_count(), 1);
        assert!(grid.page_terminals().is_empty());
    }

    #[test]
    fn add_and_remove_terminals() {
        let mut grid = HubGrid::default();
        grid.add_terminal(1);
        grid.add_terminal(2);
        grid.add_terminal(3);
        assert_eq!(grid.terminal_count(), 3);
        assert!(grid.contains_terminal(2));

        grid.remove_terminal(2);
        assert_eq!(grid.terminal_count(), 2);
        assert!(!grid.contains_terminal(2));
    }

    #[test]
    fn add_duplicate_is_noop() {
        let mut grid = HubGrid::default();
        grid.add_terminal(1);
        grid.add_terminal(1);
        assert_eq!(grid.terminal_count(), 1);
    }

    #[test]
    fn pagination_basics() {
        let mut grid = HubGrid::default();
        for i in 1..=7 {
            grid.add_terminal(i);
        }
        assert_eq!(grid.page_count(), 2);
        // Auto-navigated to page 1 (where terminal 7 is)
        assert_eq!(grid.current_page, 1);
        assert_eq!(grid.page_terminals(), &[7]);

        grid.go_to_page(0);
        assert_eq!(grid.page_terminals(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn page_navigation_wraps() {
        let mut grid = HubGrid::default();
        for i in 1..=7 {
            grid.add_terminal(i);
        }
        grid.go_to_page(0);
        grid.prev_page();
        assert_eq!(grid.current_page, 1); // wrapped to last

        grid.next_page();
        assert_eq!(grid.current_page, 0); // wrapped to first
    }

    #[test]
    fn swap_terminals() {
        let mut grid = HubGrid::default();
        grid.add_terminal(1);
        grid.add_terminal(2);
        grid.add_terminal(3);
        grid.swap_terminals(1, 3);
        assert_eq!(grid.terminals, vec![3, 2, 1]);
    }

    #[test]
    fn grid_dimensions_correct() {
        assert_eq!(grid_dimensions(0), (0, 0));
        assert_eq!(grid_dimensions(1), (1, 1));
        assert_eq!(grid_dimensions(2), (1, 2));
        assert_eq!(grid_dimensions(3), (1, 3));
        assert_eq!(grid_dimensions(4), (2, 2));
        assert_eq!(grid_dimensions(5), (2, 3));
        assert_eq!(grid_dimensions(6), (2, 3));
    }

    #[test]
    fn remove_clamps_page() {
        let mut grid = HubGrid::default();
        for i in 1..=7 {
            grid.add_terminal(i);
        }
        grid.go_to_page(1);
        // Remove terminal 7 (only item on page 2)
        grid.remove_terminal(7);
        assert_eq!(grid.page_count(), 1);
        assert_eq!(grid.current_page, 0); // clamped back
    }

    #[test]
    fn serialization_round_trips() {
        let mut grid = HubGrid::default();
        grid.add_terminal(1);
        grid.add_terminal(2);
        grid.layout = HubLayoutMode::SwimLanes;
        let json = serde_json::to_string(&grid).unwrap();
        let deserialized: HubGrid = serde_json::from_str(&json).unwrap();
        assert_eq!(grid, deserialized);
    }
}
