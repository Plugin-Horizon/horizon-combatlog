//! The console commands the mod runs for what the engine has no helper for: the boss bar timer,
//! effects on tag, damage penalties and moving a player back.
use crate::combat::PlayerId;
use crate::config::{Config, TagEffect};
use crate::text;

/// The boss bar of one player's timer.
pub fn bar_id(id: PlayerId) -> String {
    format!("combatlog:p{id:032x}")
}

/// A text component (SNBT/JSON) for `template`, `&` codes and colour tags turned into colours.
pub fn component(template: &str) -> String {
    let parts: Vec<String> = text::segments(template)
        .into_iter()
        .map(|(part, rgb)| {
            format!(
                "{{\"text\":\"{}\",\"color\":\"#{rgb:06X}\"}}",
                escape(&part)
            )
        })
        .collect();
    format!("{{\"text\":\"\",\"extra\":[{}]}}", parts.join(","))
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

fn bar_title(cfg: &Config, left: u32) -> String {
    let left = left.to_string();
    let max = cfg.combat_log.timer.duration_seconds.to_string();
    component(&text::fill(
        &cfg.combat_log.messages.timer.bossbar_title,
        &[("timeLeft", &left), ("maxTime", &max)],
    ))
}

/// Shows `name` a boss bar counting down from `left` seconds.
pub fn bar_create(cfg: &Config, id: PlayerId, name: &str, left: u32) -> Vec<String> {
    let bar = bar_id(id);
    vec![
        format!("bossbar add {bar} {}", bar_title(cfg, left)),
        format!("bossbar set {bar} color red"),
        format!(
            "bossbar set {bar} max {}",
            cfg.combat_log.timer.duration_seconds.max(1)
        ),
        format!("bossbar set {bar} value {left}"),
        format!("bossbar set {bar} players {name}"),
    ]
}

/// The bar's title and fill for `left` seconds.
pub fn bar_update(cfg: &Config, id: PlayerId, left: u32) -> Vec<String> {
    let bar = bar_id(id);
    vec![
        format!("bossbar set {bar} name {}", bar_title(cfg, left)),
        format!("bossbar set {bar} value {left}"),
    ]
}

pub fn bar_remove(id: PlayerId) -> String {
    format!("bossbar remove {}", bar_id(id))
}

/// The namespaced effect of a configured entry; None for an entry without a type or a duration of 0.
pub fn effect_kind(e: &TagEffect) -> Option<String> {
    let kind = e.kind.trim().to_ascii_lowercase();
    if kind.is_empty() || e.duration_seconds == 0 {
        return None;
    }
    Some(if kind.contains(':') {
        kind
    } else {
        format!("minecraft:{kind}")
    })
}

/// `effect give` for one configured effect; None for an effect without a type.
pub fn effect(name: &str, e: &TagEffect) -> Option<String> {
    let kind = effect_kind(e)?;
    Some(format!(
        "effect give {name} {kind} {} {} {}",
        e.duration_seconds, e.amplifier, !e.show_particles
    ))
}

/// `effect clear` for one effect CombatLog gave, when the tag ends.
pub fn effect_clear(name: &str, kind: &str) -> String {
    format!("effect clear {name} {kind}")
}

/// The configured effects for a player whose tag just started (or was renewed).
pub fn tag_effects(cfg: &Config, name: &str) -> Vec<String> {
    cfg.combat_log.tag_effects
        .iter()
        .filter_map(|e| effect(name, e))
        .collect()
}

/// The damage penalty for a refused action; None when it is off.
pub fn penalty(cfg: &Config, name: &str) -> Option<String> {
    let damage = cfg.combat_log.punishment.damage;
    (damage > 0.0).then(|| format!("damage {name} {damage} minecraft:generic"))
}

/// Moves `name` to `pos` in `dimension` (`minecraft:the_nether`).
pub fn move_back(name: &str, dimension: &str, pos: [f64; 3]) -> String {
    let [x, y, z] = pos;
    format!("execute in {dimension} run tp {name} {x} {y} {z}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_counts_down_with_the_configured_title() {
        let cfg = Config::default();
        let c = bar_create(&cfg, 0xab, "Steve", 15);
        let bar = "combatlog:p000000000000000000000000000000ab";
        assert_eq!(
            c[0],
            format!(
                "bossbar add {bar} {{\"text\":\"\",\"extra\":[{{\"text\":\"In Combat: 15s\",\"color\":\"#FF5555\"}}]}}"
            )
        );
        assert_eq!(c[2], format!("bossbar set {bar} max 15"));
        assert_eq!(c[4], format!("bossbar set {bar} players Steve"));
        assert_eq!(
            bar_update(&cfg, 0xab, 3)[1],
            format!("bossbar set {bar} value 3")
        );
        assert_eq!(bar_remove(0xab), format!("bossbar remove {bar}"));
    }

    #[test]
    fn components_escape_quotes() {
        assert_eq!(
            component("a\"b"),
            "{\"text\":\"\",\"extra\":[{\"text\":\"a\\\"b\",\"color\":\"#FFFFFF\"}]}"
        );
    }

    #[test]
    fn effects_take_bukkit_or_namespaced_names() {
        let cfg = Config::parse(
            "[[combat-log.tag-effects]]\ntype = \"SLOWNESS\"\n[[combat-log.tag-effects]]\ntype = \"minecraft:weakness\"\nduration-seconds = 5\namplifier = 2\nshow-particles = false\n[[combat-log.tag-effects]]\n",
        )
        .unwrap();
        assert_eq!(
            tag_effects(&cfg, "Alex"),
            vec![
                "effect give Alex minecraft:slowness 15 0 false".to_string(),
                "effect give Alex minecraft:weakness 5 2 true".to_string()
            ]
        );
    }

    #[test]
    fn penalty_and_move_back() {
        assert_eq!(
            penalty(&Config::default(), "Alex").as_deref(),
            Some("damage Alex 5 minecraft:generic")
        );
        let off = Config::parse("[combat-log.punishment]\ndamage = 0.0\n").unwrap();
        assert_eq!(penalty(&off, "Alex"), None);
        assert_eq!(
            move_back("Alex", "minecraft:overworld", [1.5, 64.0, -2.25]),
            "execute in minecraft:overworld run tp Alex 1.5 64 -2.25"
        );
    }
}
