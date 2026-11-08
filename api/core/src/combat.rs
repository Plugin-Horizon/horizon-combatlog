//! Who is in combat and for how long. No game types: ticks in, answers out.
use crate::config::TagMode;
use std::collections::{BTreeMap, BTreeSet};

pub type PlayerId = u128;

/// The tag mode for a hit when `attacker_ok` / `victim_ok` say who may be tagged at all (not an
/// exempt operator, not in an ignored dimension). None when nobody would be tagged.
pub fn effective_mode(mode: TagMode, attacker_ok: bool, victim_ok: bool) -> Option<TagMode> {
    let attacker = attacker_ok && matches!(mode, TagMode::Both | TagMode::Attacker);
    let victim = victim_ok && matches!(mode, TagMode::Both | TagMode::Victim);
    match (attacker, victim) {
        (true, true) => Some(TagMode::Both),
        (true, false) => Some(TagMode::Attacker),
        (false, true) => Some(TagMode::Victim),
        (false, false) => None,
    }
}

#[derive(Debug)]
struct Tag {
    until: u64,
    opponents: BTreeSet<PlayerId>,
}

#[derive(Debug, Default)]
pub struct Tagger {
    now: u64,
    tags: BTreeMap<PlayerId, Tag>,
    /// Victim -> (who hit them last, the tick that hit stops counting), for "who killed whom".
    last_hit: BTreeMap<PlayerId, (PlayerId, u64)>,
}

impl Tagger {
    /// One server tick passed. The players whose tag just ran out.
    pub fn advance(&mut self) -> Vec<PlayerId> {
        self.now += 1;
        let now = self.now;
        let over: Vec<PlayerId> = self
            .tags
            .iter()
            .filter(|(_, t)| t.until <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in &over {
            self.remove(*id);
        }
        over
    }

    pub fn is_tagged(&self, id: PlayerId) -> bool {
        self.tags.contains_key(&id)
    }

    /// Whole seconds left on a tag, rounded up (15 right after a hit); 0 when not tagged.
    pub fn seconds_left(&self, id: PlayerId) -> u32 {
        self.tags.get(&id).map_or(0, |t| {
            (t.until.saturating_sub(self.now)).div_ceil(20) as u32
        })
    }

    pub fn tagged(&self) -> Vec<PlayerId> {
        self.tags.keys().copied().collect()
    }

    /// `attacker` hurt `victim`: tag by `mode` for `ticks`, renewing a running tag. The players
    /// that were not tagged before and are now.
    pub fn hit(
        &mut self,
        attacker: PlayerId,
        victim: PlayerId,
        mode: TagMode,
        ticks: u64,
    ) -> Vec<PlayerId> {
        if attacker == victim || ticks == 0 {
            return Vec::new();
        }
        self.last_hit.insert(victim, (attacker, self.now + ticks));
        let mut fresh = Vec::new();
        let mut tag = |tagger: &mut Self, who: PlayerId, other: PlayerId| {
            let until = tagger.now + ticks;
            let new = !tagger.tags.contains_key(&who);
            let entry = tagger.tags.entry(who).or_insert_with(|| Tag {
                until,
                opponents: BTreeSet::new(),
            });
            entry.until = until;
            entry.opponents.insert(other);
            if new {
                fresh.push(who);
            }
        };
        if matches!(mode, TagMode::Both | TagMode::Attacker) {
            tag(self, attacker, victim);
        }
        if matches!(mode, TagMode::Both | TagMode::Victim) {
            tag(self, victim, attacker);
        }
        fresh
    }

    /// Tags `id` alone (no opponent) for `ticks`, renewing a running tag; with `only_renew`, a
    /// player who is not tagged stays untagged. Whether the tag is new.
    pub fn tag_alone(&mut self, id: PlayerId, ticks: u64, only_renew: bool) -> bool {
        if ticks == 0 || (only_renew && !self.is_tagged(id)) {
            return false;
        }
        let until = self.now + ticks;
        let new = !self.tags.contains_key(&id);
        self.tags
            .entry(id)
            .or_insert_with(|| Tag {
                until,
                opponents: BTreeSet::new(),
            })
            .until = until;
        new
    }

    /// Who hit `victim` last, while that hit still counts (within a tag's duration of it).
    pub fn last_attacker(&self, victim: PlayerId) -> Option<PlayerId> {
        self.last_hit
            .get(&victim)
            .filter(|(_, until)| *until > self.now)
            .map(|(attacker, _)| *attacker)
    }

    /// Ends `id`'s tag. Whether there was one.
    pub fn untag(&mut self, id: PlayerId) -> bool {
        self.remove(id)
    }

    /// `victim` died, killed by `killer` (who hit them last, if a player). The victim's own tag
    /// ends. With `untag_on_kill`, the killer's tag ends too, unless the killer still fights
    /// someone else (the original's `onOpponentDefeated`). The players untagged.
    pub fn died(
        &mut self,
        victim: PlayerId,
        killer: Option<PlayerId>,
        untag_on_kill: bool,
    ) -> Vec<PlayerId> {
        // Read before the victim is unlinked from the killer.
        let killer_ends = untag_on_kill
            && killer.filter(|&k| k != victim).is_some_and(|k| {
                self.tags
                    .get(&k)
                    .is_some_and(|t| t.opponents.iter().all(|o| *o == victim))
            });
        self.last_hit.remove(&victim);
        let mut out = Vec::new();
        if self.remove(victim) {
            out.push(victim);
        }
        if let Some(k) = killer.filter(|_| killer_ends)
            && self.remove(k)
        {
            out.push(k);
        }
        out
    }

    fn remove(&mut self, id: PlayerId) -> bool {
        let had = self.tags.remove(&id).is_some();
        for tag in self.tags.values_mut() {
            tag.opponents.remove(&id);
        }
        had
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: PlayerId = 1;
    const B: PlayerId = 2;
    const C: PlayerId = 3;

    fn ticks(t: &mut Tagger, n: u32) -> Vec<PlayerId> {
        (0..n).flat_map(|_| t.advance()).collect()
    }

    #[test]
    fn both_tags_attacker_and_victim_for_the_duration() {
        let mut t = Tagger::default();
        assert_eq!(t.hit(A, B, TagMode::Both, 300), vec![A, B]);
        assert!(t.is_tagged(A) && t.is_tagged(B));
        assert_eq!(t.seconds_left(B), 15);
        assert!(ticks(&mut t, 299).is_empty());
        assert_eq!(t.seconds_left(B), 1);
        assert_eq!(ticks(&mut t, 1), vec![A, B]);
        assert!(!t.is_tagged(A) && !t.is_tagged(B));
        assert_eq!(t.seconds_left(A), 0);
    }

    #[test]
    fn attacker_and_victim_modes_tag_one_side() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Attacker, 100);
        assert!(t.is_tagged(A) && !t.is_tagged(B));
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Victim, 100);
        assert!(!t.is_tagged(A) && t.is_tagged(B));
    }

    #[test]
    fn a_new_hit_renews_and_reports_nobody_fresh() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 100);
        ticks(&mut t, 90);
        assert!(t.hit(B, A, TagMode::Both, 100).is_empty());
        assert_eq!(t.seconds_left(A), 5);
        assert!(ticks(&mut t, 90).is_empty());
        assert!(t.is_tagged(A));
    }

    #[test]
    fn nobody_fights_themselves() {
        let mut t = Tagger::default();
        assert!(t.hit(A, A, TagMode::Both, 100).is_empty());
        assert!(!t.is_tagged(A));
    }

    #[test]
    fn a_kill_ends_the_killers_tag_when_they_have_no_other_opponent() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 300);
        assert_eq!(t.died(B, Some(A), true), vec![B, A]);
        assert!(!t.is_tagged(A) && !t.is_tagged(B));
    }

    #[test]
    fn a_killer_who_still_fights_someone_stays_tagged() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 300);
        t.hit(C, A, TagMode::Both, 300);
        // A killed B, but C is still hitting A: A stays in combat, and so does C.
        assert_eq!(t.died(B, Some(A), true), vec![B]);
        assert!(!t.is_tagged(B));
        assert!(t.is_tagged(A) && t.is_tagged(C));
    }

    #[test]
    fn a_death_without_a_player_killer_leaves_the_fighters_tagged() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 300);
        assert_eq!(t.died(B, None, true), vec![B]);
        assert!(t.is_tagged(A));
    }

    #[test]
    fn death_without_untag_on_kill_leaves_the_killer_tagged() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 300);
        assert_eq!(t.died(B, Some(A), false), vec![B]);
        assert!(t.is_tagged(A));
    }

    #[test]
    fn exempt_players_drop_out_of_the_mode() {
        use TagMode::*;
        assert_eq!(effective_mode(Both, true, true), Some(Both));
        assert_eq!(effective_mode(Both, false, true), Some(Victim));
        assert_eq!(effective_mode(Both, true, false), Some(Attacker));
        assert_eq!(effective_mode(Attacker, false, true), None);
        assert_eq!(effective_mode(Victim, true, false), None);
        assert_eq!(effective_mode(Both, false, false), None);
    }

    #[test]
    fn the_last_attacker_counts_only_while_the_hit_is_recent() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 100);
        t.hit(C, B, TagMode::Victim, 100);
        assert_eq!(t.last_attacker(B), Some(C));
        assert_eq!(t.last_attacker(A), None);
        ticks(&mut t, 100);
        assert_eq!(t.last_attacker(B), None);
        t.hit(A, B, TagMode::Both, 100);
        t.died(B, Some(A), true);
        assert_eq!(t.last_attacker(B), None);
    }

    #[test]
    fn tagging_alone_starts_or_only_renews() {
        let mut t = Tagger::default();
        assert!(!t.tag_alone(A, 100, true));
        assert!(!t.is_tagged(A));
        assert!(t.tag_alone(A, 100, false));
        ticks(&mut t, 50);
        assert!(!t.tag_alone(A, 100, true));
        assert_eq!(t.seconds_left(A), 5);
        assert_eq!(ticks(&mut t, 100), vec![A]);
    }

    #[test]
    fn untag_reports_whether_there_was_a_tag() {
        let mut t = Tagger::default();
        t.hit(A, B, TagMode::Both, 300);
        assert!(t.untag(A));
        assert!(!t.untag(A));
        assert!(t.is_tagged(B));
    }
}
