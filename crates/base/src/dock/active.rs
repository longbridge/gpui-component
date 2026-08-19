use std::collections::HashMap;

use super::layout::PanelId;

/// Tracks what each panel was last told about its active state, so a group
/// emits exactly one notification per real edge.
///
/// This is the enforcement point for the `Panel::set_active` contract: called
/// with the frame-end net state when a panel becomes, or stops being, the
/// displayed tab of its group — exactly one notification per edge, delivered
/// on the next tick after the change, never same-value repeats nor
/// false-then-true flips within one frame. A panel removed from its group is
/// NOT told `false`; `Panel::on_removed` is the deactivation signal instead. A
/// hidden panel occupying the active slot still receives `true`, even though
/// rendering falls back to the first visible panel.
///
/// Not yet wired into any panel group: `TabPanel` in `crates/ui` still owns
/// its own `notified_active` map. A later task migrates it onto this type,
/// which is why the whole type is presently unused outside its own tests.
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct ActiveTracker {
    notified: HashMap<PanelId, bool>,
    sync_scheduled: bool,
}

#[allow(dead_code)]
impl ActiveTracker {
    /// Compare the group's current membership and displayed panel against
    /// what each panel was last told, and return only the real edges,
    /// deactivations before the activation.
    ///
    /// `displayed` names at most one panel, so at most one panel can newly
    /// become active per call. Collecting deactivations separately from the
    /// single possible activation (rather than sorting the combined list)
    /// keeps "deactivations are delivered first" a structural property of
    /// this function instead of an incidental property of a sort key, and is
    /// what guarantees no panel ever observes two panels active in its group
    /// at the same time.
    pub(crate) fn reconcile(
        &mut self,
        panels: &[PanelId],
        displayed: Option<PanelId>,
    ) -> Vec<(PanelId, bool)> {
        // A panel that left the group is forgotten, not deactivated.
        self.notified.retain(|panel, _| panels.contains(panel));

        let mut deactivations = Vec::new();
        let mut activation = None;

        for panel in panels {
            let should_be_active = displayed == Some(*panel);
            let was_active = self.notified.get(panel).copied();
            if was_active == Some(should_be_active) {
                continue;
            }
            if !should_be_active && was_active.is_none() {
                // Never announce `false` to a panel that was never told `true`.
                self.notified.insert(*panel, false);
                continue;
            }

            self.notified.insert(*panel, should_be_active);
            if should_be_active {
                debug_assert!(
                    activation.is_none(),
                    "`displayed` names at most one panel, so at most one activation per reconcile"
                );
                activation = Some((*panel, true));
            } else {
                deactivations.push((*panel, false));
            }
        }

        deactivations.extend(activation);
        deactivations
    }

    /// Record what a panel was last told, without emitting anything. Used
    /// when a panel moves between groups while displayed, so the move does
    /// not look like an edge to the destination group.
    pub(crate) fn seed(&mut self, panel: PanelId, active: bool) {
        self.notified.insert(panel, active);
    }

    #[cfg(test)]
    pub(crate) fn tracks(&self, panel: PanelId) -> bool {
        self.notified.contains_key(&panel)
    }

    /// Returns whether a sync needs scheduling; the caller defers the actual
    /// `reconcile` to frame end so within-frame churn nets out to one
    /// delivery per edge.
    pub(crate) fn schedule_sync(&mut self) -> bool {
        let needed = !self.sync_scheduled;
        self.sync_scheduled = true;
        needed
    }

    /// Marks the deferred sync as delivered, so a future change schedules a
    /// new one. Callers should invoke this alongside `reconcile` (typically
    /// just before it) when the scheduled sync runs.
    pub(crate) fn sync_finished(&mut self) {
        self.sync_scheduled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(n: u64) -> PanelId {
        PanelId::from_u64(n)
    }

    #[test]
    fn the_first_reconcile_announces_the_displayed_panel_only() {
        let mut tracker = ActiveTracker::default();
        let changes = tracker.reconcile(&[panel(1), panel(2)], Some(panel(1)));

        assert_eq!(changes, vec![(panel(1), true)]);
    }

    #[test]
    fn switching_reports_the_old_panel_false_then_the_new_panel_true() {
        let mut tracker = ActiveTracker::default();
        tracker.reconcile(&[panel(1), panel(2)], Some(panel(1)));

        let changes = tracker.reconcile(&[panel(1), panel(2)], Some(panel(2)));

        assert_eq!(changes, vec![(panel(1), false), (panel(2), true)]);
    }

    #[test]
    fn reselecting_the_displayed_panel_is_silent() {
        let mut tracker = ActiveTracker::default();
        tracker.reconcile(&[panel(1)], Some(panel(1)));

        assert!(tracker.reconcile(&[panel(1)], Some(panel(1))).is_empty());
    }

    #[test]
    fn a_removed_panel_is_forgotten_rather_than_told_false() {
        let mut tracker = ActiveTracker::default();
        tracker.reconcile(&[panel(1), panel(2)], Some(panel(1)));

        let changes = tracker.reconcile(&[panel(2)], Some(panel(2)));

        assert_eq!(
            changes,
            vec![(panel(2), true)],
            "panel 1 left the group; on_removed is its deactivation signal"
        );
        assert!(!tracker.tracks(panel(1)));
    }

    #[test]
    fn seeding_a_moved_panel_prevents_a_spurious_reactivation() {
        let mut tracker = ActiveTracker::default();
        tracker.seed(panel(1), true);

        assert!(
            tracker.reconcile(&[panel(1)], Some(panel(1))).is_empty(),
            "a panel dragged in while displayed must not be told true twice"
        );
    }

    #[test]
    fn no_panel_displayed_deactivates_the_previous_one() {
        let mut tracker = ActiveTracker::default();
        tracker.reconcile(&[panel(1)], Some(panel(1)));

        let changes = tracker.reconcile(&[panel(1)], None);

        assert_eq!(changes, vec![(panel(1), false)]);
    }
}
