//! What a tagged player may not do: glide, teleport, riptide. The game glue asks before (or,
//! where the engine only tells afterwards, right after) the action and carries out the answer.
use crate::combatlog_core::combat::PlayerId;
use crate::combatlog_core::config::Config;
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
    /// A player launched with riptide at tick `now`. As the original: in combat with
    /// `riptide-stop`, the first launch is allowed and starts a `riptide-cooldown-ms` window; a
    /// launch inside that window is stopped (the window is not renewed by the stop).
    pub fn riptide(&mut self, cfg: &Config, id: PlayerId, tagged: bool, now: u64) -> bool {
        if !cfg.combat_log.restrictions.riptide.stop || !tagged {
            return false;
        }
        let window_open = self
            .riptide_until
            .get(&id)
            .is_some_and(|until| *until > now);
        if window_open {
            return true;
        }
        self.riptide_until
            .insert(id, now + cfg.riptide_cooldown_ticks());
        false
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
    fn the_first_riptide_in_combat_is_allowed_and_a_second_within_the_cooldown_is_stopped() {
        let c = cfg("[combat-log.restrictions.riptide]\nstop = true\ncooldown = 1000\n");
        let mut g = Guard::default();
        // Not in combat: nothing happens and nothing starts.
        assert!(!g.riptide(&c, 1, false, 0));
        assert!(!g.riptide(&c, 1, true, 10));
        // Inside the 20-tick window: stopped, and the stop does not renew the window.
        assert!(g.riptide(&c, 1, true, 29));
        assert!(!g.riptide(&c, 1, true, 30));
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
