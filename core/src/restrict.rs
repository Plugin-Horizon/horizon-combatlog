//! What a tagged player may not do: glide, teleport, riptide. The game glue asks before (or,
//! where the engine only tells afterwards, right after) the action and carries out the answer.
use crate::combat::PlayerId;
use crate::config::Config;
use std::collections::BTreeMap;

/// An action the restrictions look at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Starting to glide with an elytra.
    Elytra,
    EnderPearl,
    ChorusFruit,
    EndGateway,
    /// `/tp` or `/teleport` moved the player (heard after the move).
    CommandTeleport,
}

/// The message template for a refused `action` of a player who is `tagged`; None when it is
/// allowed.
pub fn refused(cfg: &Config, action: Action, tagged: bool) -> Option<&str> {
    let r = &cfg.combat_log.restrictions;
    if !tagged {
        return None;
    }
    match action {
        Action::Elytra => r
            .elytra
            .disabled_in_combat
            .then_some(cfg.combat_log.messages.elytra_use_denied.as_str()),
        Action::EnderPearl if r.teleporting.allow_enderpearl => None,
        Action::EnderPearl | Action::ChorusFruit | Action::EndGateway | Action::CommandTeleport => {
            r.teleporting
                .disabled_in_combat
                .then_some(cfg.combat_log.messages.teleporting_denied.as_str())
        }
    }
}

/// State that outlives one hook call: riptide locks and the teleports the mod itself makes to
/// undo a `/tp` (which the engine reports like any other).
#[derive(Debug, Default)]
pub struct Guard {
    riptide_until: BTreeMap<PlayerId, u64>,
    undoing: BTreeMap<PlayerId, [f64; 3]>,
}

impl Guard {
    /// A player launched with riptide at tick `now`. Whether to stop it: in combat with
    /// `riptide-stop`, or while an earlier stop's `riptide-cooldown-ms` lock still runs. A
    /// stop (re)starts the lock.
    pub fn riptide(&mut self, cfg: &Config, id: PlayerId, tagged: bool, now: u64) -> bool {
        if !cfg.combat_log.restrictions.riptide.stop {
            return false;
        }
        let locked = self
            .riptide_until
            .get(&id)
            .is_some_and(|until| *until > now);
        if !tagged && !locked {
            self.riptide_until.remove(&id);
            return false;
        }
        self.riptide_until
            .insert(id, now + cfg.riptide_cooldown_ticks());
        true
    }

    /// `/tp` moved a player to `to`. Whether to move them back: not when this is the move back
    /// the mod asked for itself.
    pub fn teleported(&mut self, cfg: &Config, id: PlayerId, tagged: bool, to: [f64; 3]) -> bool {
        if let Some(back) = self.undoing.remove(&id)
            && near(back, to)
        {
            return false;
        }
        refused(cfg, Action::CommandTeleport, tagged).is_some()
    }

    /// The mod is about to move `id` back to `to`; the report of that move is not refused again.
    pub fn undoing(&mut self, id: PlayerId, to: [f64; 3]) {
        self.undoing.insert(id, to);
    }

    /// A player left: nothing of theirs is kept.
    pub fn forget(&mut self, id: PlayerId) {
        self.riptide_until.remove(&id);
        self.undoing.remove(&id);
    }
}

fn near(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(toml: &str) -> Config {
        Config::parse(toml).unwrap()
    }

    #[test]
    fn the_default_config_refuses_only_the_elytra() {
        let c = Config::default();
        assert!(refused(&c, Action::Elytra, true).is_some());
        for a in [
            Action::EnderPearl,
            Action::ChorusFruit,
            Action::EndGateway,
            Action::CommandTeleport,
        ] {
            assert_eq!(refused(&c, a, true), None, "{a:?}");
        }
        assert!(!Guard::default().riptide(&c, 1, true, 0));
    }

    #[test]
    fn teleports_are_refused_in_combat_only() {
        let c = cfg("[combat-log.restrictions.teleporting]\ndisabled-in-combat = true\n");
        assert_eq!(
            refused(&c, Action::EnderPearl, true),
            Some("&dYou can't teleport in combat")
        );
        assert!(refused(&c, Action::ChorusFruit, true).is_some());
        assert!(refused(&c, Action::EndGateway, true).is_some());
        assert_eq!(refused(&c, Action::EnderPearl, false), None);
        assert!(refused(&c, Action::Elytra, true).is_some());
    }

    #[test]
    fn allow_enderpearl_keeps_pearls_but_not_other_teleports() {
        let c = cfg("[combat-log.restrictions.teleporting]\ndisabled-in-combat = true\nallow-enderpearl = true\n");
        assert_eq!(refused(&c, Action::EnderPearl, true), None);
        assert!(refused(&c, Action::ChorusFruit, true).is_some());
    }

    #[test]
    fn elytra_has_its_own_switch_and_message() {
        let c = Config::default();
        assert_eq!(
            refused(&c, Action::Elytra, true),
            Some("&dYou can't use elytra in combat")
        );
        assert_eq!(refused(&c, Action::EnderPearl, true), None);
        let off = cfg("[combat-log.restrictions.elytra]\ndisabled-in-combat = false\n");
        assert_eq!(refused(&off, Action::Elytra, true), None);
    }

    #[test]
    fn a_stopped_riptide_locks_the_trident_past_the_tag() {
        let c = cfg("[combat-log.restrictions.riptide]\nstop = true\ncooldown = 1000\n");
        let mut g = Guard::default();
        assert!(!g.riptide(&c, 1, false, 0));
        assert!(g.riptide(&c, 1, true, 10));
        // The tag ran out, the 20-tick lock did not.
        assert!(g.riptide(&c, 1, false, 25));
        // That stop renewed the lock to tick 45.
        assert!(g.riptide(&c, 1, false, 44));
        assert!(!g.riptide(&c, 1, false, 64));
    }

    #[test]
    fn the_mods_own_move_back_is_not_refused() {
        let c = cfg("[combat-log.restrictions.teleporting]\ndisabled-in-combat = true\n");
        let mut g = Guard::default();
        assert!(g.teleported(&c, 1, true, [100.0, 64.0, 100.0]));
        g.undoing(1, [0.5, 64.0, 0.5]);
        assert!(!g.teleported(&c, 1, true, [0.5, 64.0, 0.5]));
        // Only once: a later /tp to the same spot is refused again.
        assert!(g.teleported(&c, 1, true, [0.5, 64.0, 0.5]));
        assert!(!g.teleported(&c, 2, false, [0.0; 3]));
    }
}
