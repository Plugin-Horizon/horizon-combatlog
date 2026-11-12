//! `/combatlog reload | status <player> | untag <player>`: parsing and the answers.
use crate::config::Config;
use crate::text::fill;

pub const LABEL: &str = "combatlog";

/// The usages registered with the server (completion and operator level). Every sub-command is listed bare as well,
/// so a player who types it without a name gets the usage lines instead of the client's syntax error. The name is a
/// `player_name` (any word; the lookup only finds online players), so a miss gets the "not found" answer.
pub const USAGES: [&str; 6] = [
    "combatlog",
    "combatlog reload",
    "combatlog status",
    "combatlog status <player:player_name>",
    "combatlog untag",
    "combatlog untag <player:player_name>",
];

#[derive(Debug, PartialEq, Eq)]
pub enum Admin {
    Usage,
    /// `status` or `untag` without a player: only that sub-command's usage line, as the original.
    SubUsage(&'static str),
    Reload,
    Status(String),
    Untag(String),
}

/// The command in `line` (without its slash); None when it is not `/combatlog`.
pub fn parse(line: &str) -> Option<Admin> {
    let mut words = line.split_whitespace();
    let label = words.next()?.to_ascii_lowercase();
    if label != LABEL && label != format!("{LABEL}:{LABEL}") {
        return None;
    }
    let sub = words.next().map(str::to_ascii_lowercase);
    let arg = words.next().map(str::to_owned);
    Some(match (sub.as_deref(), arg) {
        (Some("reload"), _) => Admin::Reload,
        (Some("status"), Some(p)) => Admin::Status(p),
        (Some("untag"), Some(p)) => Admin::Untag(p),
        (Some("status"), None) => Admin::SubUsage("status"),
        (Some("untag"), None) => Admin::SubUsage("untag"),
        _ => Admin::Usage,
    })
}

/// The one usage line of `status` or `untag`.
pub fn sub_usage(cfg: &Config, which: &str) -> String {
    let m = &cfg.command_messages;
    let line = if which == "status" {
        &m.usage_status
    } else {
        &m.usage_untag
    };
    fill(line, &[("label", LABEL)])
}

pub fn usage(cfg: &Config) -> Vec<String> {
    let m = &cfg.command_messages;
    let l = [("label", LABEL)];
    vec![
        m.usage_header.clone(),
        fill(&m.usage_reload, &l),
        fill(&m.usage_status, &l),
        fill(&m.usage_untag, &l),
    ]
}

/// What `/combatlog status` shows about a player.
pub struct Status<'a> {
    pub player: &'a str,
    pub dimension: &'a str,
    pub game_mode: i32,
    pub seconds_left: Option<u32>,
    pub ignored_dimension: bool,
}

pub fn game_mode_name(mode: i32) -> &'static str {
    match mode {
        0 => "SURVIVAL",
        1 => "CREATIVE",
        2 => "ADVENTURE",
        3 => "SPECTATOR",
        _ => "UNKNOWN",
    }
}

pub fn status(cfg: &Config, s: &Status<'_>) -> Vec<String> {
    let m = &cfg.command_messages;
    let yes_no = |b: bool| {
        if b {
            m.value_yes.as_str()
        } else {
            m.value_no.as_str()
        }
    };
    let left = s
        .seconds_left
        .map_or(m.value_not_applicable.clone(), |n| format!("{n}s"));
    vec![
        fill(&m.status_title, &[("player", s.player)]),
        fill(&m.status_world, &[("world", s.dimension)]),
        fill(
            &m.status_gamemode,
            &[("gamemode", game_mode_name(s.game_mode))],
        ),
        fill(
            &m.status_in_combat,
            &[("value", yes_no(s.seconds_left.is_some()))],
        ),
        fill(&m.status_time_left, &[("value", &left)]),
        fill(
            &m.status_ignored_world,
            &[("value", yes_no(s.ignored_dimension))],
        ),
        // Neither the combat zone nor a WorldGuard bridge exists in this port.
        fill(&m.status_combat_zone_enabled, &[("value", yes_no(false))]),
        fill(&m.status_worldguard_active, &[("value", yes_no(false))]),
    ]
}

pub fn not_found(cfg: &Config, player: &str) -> String {
    fill(
        &cfg.command_messages.player_not_found,
        &[("player", player)],
    )
}

pub fn untagged(cfg: &Config, player: &str, was_tagged: bool) -> String {
    let m = &cfg.command_messages;
    fill(
        if was_tagged {
            &m.untag_success
        } else {
            &m.untag_not_in_combat
        },
        &[("player", player)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_three_subcommands_and_falls_back_to_usage() {
        assert_eq!(parse("combatlog reload"), Some(Admin::Reload));
        assert_eq!(
            parse("CombatLog status Steve"),
            Some(Admin::Status("Steve".into()))
        );
        assert_eq!(
            parse("combatlog:combatlog untag Alex"),
            Some(Admin::Untag("Alex".into()))
        );
        assert_eq!(parse("combatlog status"), Some(Admin::SubUsage("status")));
        assert_eq!(parse("combatlog untag"), Some(Admin::SubUsage("untag")));
        assert_eq!(parse("combatlog"), Some(Admin::Usage));
        assert_eq!(parse("combatlog fly"), Some(Admin::Usage));
        assert_eq!(parse("tpa Steve"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn usage_names_the_label() {
        let lines = usage(&Config::default());
        assert_eq!(lines[0], "<yellow>Usage:");
        assert_eq!(lines[2], "<gray>/combatlog status <player>");
    }

    #[test]
    fn status_of_a_tagged_and_an_untagged_player() {
        let cfg = Config::default();
        let tagged = status(
            &cfg,
            &Status {
                player: "Steve",
                dimension: "minecraft:overworld",
                game_mode: 0,
                seconds_left: Some(9),
                ignored_dimension: false,
            },
        );
        assert_eq!(tagged[0], "<gold>Combat status for <white>Steve");
        assert_eq!(tagged[2], "<gray>Gamemode: <white>SURVIVAL");
        assert_eq!(tagged[3], "<gray>In combat: <white>yes");
        assert_eq!(tagged[4], "<gray>Time left: <white>9s");
        let calm = status(
            &cfg,
            &Status {
                player: "Alex",
                dimension: "minecraft:the_end",
                game_mode: 1,
                seconds_left: None,
                ignored_dimension: true,
            },
        );
        assert_eq!(calm[4], "<gray>Time left: <white>n/a");
        assert_eq!(calm[5], "<gray>Ignored world: <white>yes");
        assert_eq!(
            untagged(&cfg, "Alex", false),
            "<yellow>Player <white>Alex <yellow>was not in combat."
        );
        assert_eq!(not_found(&cfg, "Bob"), "<red>Player not found: <white>Bob");
    }
}
