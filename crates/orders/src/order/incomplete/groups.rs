use super::tied_rank_ref::TiedRankRef;

// Splits an order up into its rankings
pub struct GroupIterator<'a> {
    pub(crate) order: TiedRankRef<'a>,
}

impl<'a> Iterator for GroupIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        if self.order.is_empty() {
            return None;
        }
        let (group, order) = self.order.split_winner_group();
        self.order = order;
        debug_assert!(!group.is_empty());
        Some(group)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.order.is_empty() {
            // We're done
            (0, Some(0))
        } else {
            // We could have one group if all elements are tied, or one group for each
            // element
            (1, Some(self.order.len()))
        }
    }
}
