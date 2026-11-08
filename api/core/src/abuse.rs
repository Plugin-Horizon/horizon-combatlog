//! Anti-kill-abuse: a player who kills the same player more than `max-kills` times within
//! `remove-kill-after-seconds` is punished. Counts are per killer and victim pair, as in the
//! original (`killHistory`: killer, then victim, then the kill times). Every kill is forgotten
//! that long after it.
use crate::combat::PlayerId;
use crate::config::AntiKillAbuse;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Default)]
pub struct KillLog {
    kills: BTreeMap<(PlayerId, PlayerId), VecDeque<u64>>,
}

impl KillLog {
    /// `killer` killed `victim` at tick `now`. Whether that makes the killer a kill abuser for
    /// this victim; their count for that victim then starts over, so one burst is punished once.
    pub fn record(&mut self, cfg: &AntiKillAbuse, killer: PlayerId, victim: PlayerId, now: u64) -> bool {
        if !cfg.enabled {
            return false;
        }
        let window = u64::from(cfg.limits.remove_kill_after_seconds) * 20;
        let kills = self.kills.entry((killer, victim)).or_default();
        while kills
            .front()
            .is_some_and(|t| now.saturating_sub(*t) >= window)
        {
            kills.pop_front();
        }
        kills.push_back(now);
        if kills.len() > cfg.limits.max_kills as usize {
            kills.clear();
            return true;
        }
        false
    }

    /// Kills of `victim` by `killer` that still count at `now`.
    pub fn count(&self, cfg: &AntiKillAbuse, killer: PlayerId, victim: PlayerId, now: u64) -> usize {
        let window = u64::from(cfg.limits.remove_kill_after_seconds) * 20;
        self.kills.get(&(killer, victim)).map_or(0, |k| {
            k.iter()
                .filter(|t| now.saturating_sub(**t) < window)
                .count()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> AntiKillAbuse {
        AntiKillAbuse {
            enabled: true,
            ..AntiKillAbuse::default()
        }
    }

    #[test]
    fn off_never_punishes() {
        let mut log = KillLog::default();
        let off = AntiKillAbuse {
            enabled: false,
            ..AntiKillAbuse::default()
        };
        assert!((0..20).all(|t| !log.record(&off, 1, 2, t)));
    }

    #[test]
    fn the_sixth_kill_within_a_minute_is_abuse_and_starts_the_count_over() {
        let cfg = on();
        let mut log = KillLog::default();
        for t in 0..5 {
            assert!(!log.record(&cfg, 1, 2, t * 100));
        }
        assert!(log.record(&cfg, 1, 2, 500));
        assert_eq!(log.count(&cfg, 1, 2, 500), 0);
        assert!(!log.record(&cfg, 1, 2, 600));
    }

    #[test]
    fn kills_older_than_the_window_do_not_count() {
        let cfg = on();
        let mut log = KillLog::default();
        for t in 0..5 {
            log.record(&cfg, 1, 2, t * 300);
        }
        // Kills at 0 and 300 are 60 s (1200 ticks) old at 1500: three still count, plus this one.
        assert!(!log.record(&cfg, 1, 2, 1500));
        assert_eq!(log.count(&cfg, 1, 2, 1500), 4);
    }

    #[test]
    fn each_victim_has_their_own_count() {
        let cfg = on();
        let mut log = KillLog::default();
        for t in 0..5 {
            assert!(!log.record(&cfg, 1, 2, t * 100));
        }
        // Six kills of three different victims are not abuse: the count is per victim.
        assert!(!log.record(&cfg, 1, 3, 500));
        assert!(!log.record(&cfg, 1, 4, 510));
        assert_eq!(log.count(&cfg, 1, 3, 510), 1);
        assert_eq!(log.count(&cfg, 2, 2, 510), 0);
    }
}
