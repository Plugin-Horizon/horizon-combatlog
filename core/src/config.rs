//! `config/combatlog-common.toml`: written with these defaults when missing, reloaded when saved.
//! The key paths and defaults are those of the Bukkit/Paper plugin CombatLog 1.20 (its config.yml
//! and messages.yml), written as TOML tables. Port-only switches are in `[admin]` and `[exemptions]`.
use serde::Deserialize;

pub const DEFAULT_TOML: &str = r##"# CombatLog for Horizon. Saved changes are picked up within a second.
# Key paths follow the original plugin's config.yml and messages.yml (see the README).
config-version = 4

# Dimensions (or their part after the colon) where nothing is tagged.
ignored-worlds = ["ffa"]

[combat-log.timer]
# Seconds a tag lasts; every hit renews it.
duration-seconds = 15
# Where the countdown shows: "actionbar" or "bossbar".
display = "actionbar"

[combat-log.restrictions.elytra]
# Starting to glide with an elytra in combat is answered and punished. The engine cannot stop
# the glide yet, so the player keeps gliding (see the README).
disabled-in-combat = true

[combat-log.restrictions.teleporting]
# Teleports in combat are refused: ender pearls, chorus fruit, end gateways, /tp.
disabled-in-combat = false
# With disabled-in-combat, ender pearls still work.
allow-enderpearl = false

[combat-log.restrictions.mending]
# A tagged player's Mending repairs nothing (the experience is not spent).
disabled-in-combat = true

[combat-log.restrictions.riptide]
# A riptide launch in combat is stopped (the player is put back where it started) and the
# trident is locked for cooldown milliseconds, also after the tag ran out.
stop = false
cooldown = 10000

[combat-log.punishment]
# Health points taken from a tagged player who tries a blocked command or a refused action (0 = off).
damage = 5.0

[combat-log.punishment.logout]
# What logging out while tagged does: "none", "kill" or "commands".
# "kill" kills the player on their next join, where they logged out (see the README).
mode = "none"
# Console commands for mode "commands"; {player} is the player's name.
commands = []

[combat-log.triggers]
# Who is tagged when a player hurts another player: "both", "attacker" or "victim".
tag-mode = "both"
# Only tag when the hurt player is at this health or lower (0 = always; 20 = full health).
start-below-health = 0.0
# Killing a player ends the combat of those who were fighting them.
untag-on-kill = true
# Explosions (end crystals, block explosions) set combat in the original; not ported yet (GAPS in the README).

[combat-log.triggers.enderpearl]
# A landing ender pearl tags its thrower.
set-combat-on-land = true
# With the line above: only renew a running tag, never start one.
only-if-already-in-combat = false

# Effects given to a player when their tag starts, e.g.
# [[combat-log.tag-effects]]
# type = "slowness"
# duration-seconds = 15
# amplifier = 0
# show-particles = true

[combat-log.messages]
# "&" colour codes or MiniMessage colour tags (<red>). {player}, {timeLeft}, {maxTime}.
combat-log = "&c{player} has combat logged!"
elytra-use-denied = "&dYou can't use elytra in combat"
teleporting-denied = "&dYou can't teleport in combat"
blocked-command = "&cYou can't use this command in combat"
riptide-denied = "&cYou can't use riptide in combat"
# region-entry-denied and afk-title belong to WorldGuard and an AFK plugin: not ported.

[combat-log.messages.timer]
actionbar = "&c{timeLeft}/{maxTime}"
bossbar-title = "&cIn Combat: {timeLeft}s"

[command-restrictions]
# "blacklist" blocks the listed commands, "whitelist" blocks all but the listed ones.
mode = "blacklist"
commands = ["tpa"]

[anti-kill-abuse]
# A player who kills more than limits.max-kills players within limits.remove-kill-after-seconds
# is punished with these console commands ({killer} is their name). The kill count then starts over.
enabled = true
punish-commands = ["kick {killer} Kill-Abuse detected!", "ban {killer}"]

[anti-kill-abuse.limits]
max-kills = 5
remove-kill-after-seconds = 60

[exemptions]
# Port-only: stands in for the permission combatlog.bypass (default: operators). Players with an
# operator level of at least this are never tagged (0 = nobody is exempt).
bypass-min-permission = 1

[admin]
# Port-only: stands in for the permission combatlog.admin (default: operators), needed for
# /combatlog reload|status|untag.
min-permission = 1

[command-messages]
# The answers of /combatlog. {label}, {player}, {world}, {gamemode}, {value}.
usage-header = "<yellow>Usage:"
usage-reload = "<gray>/{label} reload"
usage-status = "<gray>/{label} status <player>"
usage-untag = "<gray>/{label} untag <player>"
reload-success = "<green>CombatLog config and messages reloaded."
reload-failed = "<red>Failed to reload CombatLog. Check console for details."
player-not-found = "<red>Player not found: <white>{player}"
untag-success = "<green>Removed combat tag from <white>{player}"
untag-not-in-combat = "<yellow>Player <white>{player} <yellow>was not in combat."
status-title = "<gold>Combat status for <white>{player}"
status-world = "<gray>World: <white>{world}"
status-gamemode = "<gray>Gamemode: <white>{gamemode}"
status-in-combat = "<gray>In combat: <white>{value}"
status-time-left = "<gray>Time left: <white>{value}"
status-ignored-world = "<gray>Ignored world: <white>{value}"
status-combat-zone-enabled = "<gray>Combat zone enabled: <white>{value}"
status-worldguard-active = "<gray>WorldGuard active: <white>{value}"
value-yes = "yes"
value-no = "no"
value-not-applicable = "n/a"
"##;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagMode {
    #[default]
    Both,
    Attacker,
    Victim,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogoutMode {
    #[default]
    None,
    Kill,
    Commands,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListMode {
    #[default]
    Blacklist,
    Whitelist,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    #[default]
    Actionbar,
    Bossbar,
}

/// `combat-log`: the original's config.yml and the messages of its messages.yml.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CombatLog {
    pub timer: Timer,
    pub restrictions: Restrictions,
    pub punishment: Punishment,
    pub triggers: Triggers,
    pub tag_effects: Vec<TagEffect>,
    pub messages: Messages,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Timer {
    pub duration_seconds: u32,
    pub display: Display,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Restrictions {
    pub elytra: Elytra,
    pub teleporting: Teleporting,
    pub mending: Mending,
    pub riptide: Riptide,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Elytra {
    pub disabled_in_combat: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Teleporting {
    pub disabled_in_combat: bool,
    pub allow_enderpearl: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Mending {
    pub disabled_in_combat: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Riptide {
    pub stop: bool,
    /// Milliseconds.
    pub cooldown: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Punishment {
    pub damage: f32,
    pub logout: Logout,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Logout {
    pub mode: LogoutMode,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Triggers {
    pub tag_mode: TagMode,
    pub start_below_health: f32,
    pub untag_on_kill: bool,
    pub enderpearl: Enderpearl,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Enderpearl {
    pub set_combat_on_land: bool,
    pub only_if_already_in_combat: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CommandRestrictions {
    pub mode: ListMode,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AntiKillAbuse {
    pub enabled: bool,
    pub limits: Limits,
    pub punish_commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Limits {
    pub max_kills: u32,
    pub remove_kill_after_seconds: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Exemptions {
    pub bypass_min_permission: u8,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Admin {
    pub min_permission: u8,
}

/// An effect given when a tag starts. `type` is an effect id (`slowness`, `minecraft:slowness`;
/// the Bukkit spelling `SLOWNESS` works too).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default, rename_all = "kebab-case")]
pub struct TagEffect {
    #[serde(rename = "type")]
    pub kind: String,
    pub duration_seconds: u32,
    pub amplifier: u8,
    pub show_particles: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Messages {
    pub combat_log: String,
    pub elytra_use_denied: String,
    pub teleporting_denied: String,
    pub blocked_command: String,
    pub riptide_denied: String,
    pub timer: TimerMessages,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct TimerMessages {
    pub actionbar: String,
    pub bossbar_title: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub combat_log: CombatLog,
    pub ignored_worlds: Vec<String>,
    pub command_restrictions: CommandRestrictions,
    pub anti_kill_abuse: AntiKillAbuse,
    pub exemptions: Exemptions,
    pub admin: Admin,
    pub command_messages: CommandMessages,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CommandMessages {
    pub usage_header: String,
    pub usage_reload: String,
    pub usage_status: String,
    pub usage_untag: String,
    pub reload_success: String,
    pub reload_failed: String,
    pub player_not_found: String,
    pub untag_success: String,
    pub untag_not_in_combat: String,
    pub status_title: String,
    pub status_world: String,
    pub status_gamemode: String,
    pub status_in_combat: String,
    pub status_time_left: String,
    pub status_ignored_world: String,
    pub status_combat_zone_enabled: String,
    pub status_worldguard_active: String,
    pub value_yes: String,
    pub value_no: String,
    pub value_not_applicable: String,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            duration_seconds: 15,
            display: Display::Actionbar,
        }
    }
}
impl Default for Elytra {
    fn default() -> Self {
        Self {
            disabled_in_combat: true,
        }
    }
}
impl Default for Teleporting {
    fn default() -> Self {
        Self {
            disabled_in_combat: false,
            allow_enderpearl: false,
        }
    }
}
impl Default for Mending {
    fn default() -> Self {
        Self {
            disabled_in_combat: true,
        }
    }
}
impl Default for Riptide {
    fn default() -> Self {
        Self {
            stop: false,
            cooldown: 10_000,
        }
    }
}
impl Default for Triggers {
    fn default() -> Self {
        Self {
            tag_mode: TagMode::Both,
            start_below_health: 0.0,
            untag_on_kill: true,
            enderpearl: Enderpearl::default(),
        }
    }
}
impl Default for Enderpearl {
    fn default() -> Self {
        Self {
            set_combat_on_land: true,
            only_if_already_in_combat: false,
        }
    }
}
impl Default for Punishment {
    fn default() -> Self {
        Self {
            damage: 5.0,
            logout: Logout::default(),
        }
    }
}
impl Default for CommandRestrictions {
    fn default() -> Self {
        Self {
            mode: ListMode::Blacklist,
            commands: vec!["tpa".into()],
        }
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_kills: 5,
            remove_kill_after_seconds: 60,
        }
    }
}
impl Default for AntiKillAbuse {
    fn default() -> Self {
        Self {
            enabled: true,
            limits: Limits::default(),
            punish_commands: vec![
                "kick {killer} Kill-Abuse detected!".into(),
                "ban {killer}".into(),
            ],
        }
    }
}
impl Default for Exemptions {
    fn default() -> Self {
        Self {
            bypass_min_permission: 1,
        }
    }
}
impl Default for Admin {
    fn default() -> Self {
        Self { min_permission: 1 }
    }
}
impl Default for TagEffect {
    fn default() -> Self {
        Self {
            kind: String::new(),
            duration_seconds: 15,
            amplifier: 0,
            show_particles: true,
        }
    }
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            combat_log: "&c{player} has combat logged!".into(),
            elytra_use_denied: "&dYou can't use elytra in combat".into(),
            teleporting_denied: "&dYou can't teleport in combat".into(),
            blocked_command: "&cYou can't use this command in combat".into(),
            riptide_denied: "&cYou can't use riptide in combat".into(),
            timer: TimerMessages::default(),
        }
    }
}
impl Default for TimerMessages {
    fn default() -> Self {
        Self {
            actionbar: "&c{timeLeft}/{maxTime}".into(),
            bossbar_title: "&cIn Combat: {timeLeft}s".into(),
        }
    }
}
impl Default for CommandMessages {
    fn default() -> Self {
        let s = |t: &str| t.to_owned();
        Self {
            usage_header: s("<yellow>Usage:"),
            usage_reload: s("<gray>/{label} reload"),
            usage_status: s("<gray>/{label} status <player>"),
            usage_untag: s("<gray>/{label} untag <player>"),
            reload_success: s("<green>CombatLog config and messages reloaded."),
            reload_failed: s("<red>Failed to reload CombatLog. Check console for details."),
            player_not_found: s("<red>Player not found: <white>{player}"),
            untag_success: s("<green>Removed combat tag from <white>{player}"),
            untag_not_in_combat: s("<yellow>Player <white>{player} <yellow>was not in combat."),
            status_title: s("<gold>Combat status for <white>{player}"),
            status_world: s("<gray>World: <white>{world}"),
            status_gamemode: s("<gray>Gamemode: <white>{gamemode}"),
            status_in_combat: s("<gray>In combat: <white>{value}"),
            status_time_left: s("<gray>Time left: <white>{value}"),
            status_ignored_world: s("<gray>Ignored world: <white>{value}"),
            status_combat_zone_enabled: s("<gray>Combat zone enabled: <white>{value}"),
            status_worldguard_active: s("<gray>WorldGuard active: <white>{value}"),
            value_yes: s("yes"),
            value_no: s("no"),
            value_not_applicable: s("n/a"),
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// Ticks a tag lasts (20 a second).
    pub fn duration_ticks(&self) -> u64 {
        u64::from(self.combat_log.timer.duration_seconds) * 20
    }

    /// Whether a player's command is refused in combat. As the original's `extractCommand`: the
    /// first word without a slash, in any case, matches a listed name either whole (`essentials:tpa`)
    /// or without its namespace (`tpa`).
    pub fn command_blocked(&self, command: &str) -> bool {
        let word = command
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let base = word.split_once(':').map_or(word.as_str(), |(_, base)| base);
        let listed = self
            .command_restrictions
            .commands
            .iter()
            .map(|c| c.to_ascii_lowercase())
            .any(|c| c == word || c == base);
        match self.command_restrictions.mode {
            ListMode::Blacklist => listed,
            ListMode::Whitelist => !listed,
        }
    }

    /// Whether nothing is tagged in `dimension` (`minecraft:overworld`).
    pub fn dimension_ignored(&self, dimension: &str) -> bool {
        let short = dimension.rsplit(':').next().unwrap_or(dimension);
        self.ignored_worlds
            .iter()
            .any(|d| d == dimension || d == short)
    }

    /// Ticks a riptide stays locked after it was stopped (20 a second, rounded up).
    pub fn riptide_cooldown_ticks(&self) -> u64 {
        self.combat_log.restrictions.riptide.cooldown.div_ceil(50)
    }

    /// Whether an operator of `permission` is exempt.
    pub fn bypasses(&self, permission: u8) -> bool {
        let at = self.exemptions.bypass_min_permission;
        at > 0 && permission >= at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_default_parses_to_the_defaults() {
        let parsed = Config::parse(DEFAULT_TOML).unwrap();
        let plain = Config::default();
        let (p, d) = (&parsed.combat_log, &plain.combat_log);
        assert_eq!(p.timer.duration_seconds, d.timer.duration_seconds);
        assert_eq!(p.punishment.logout.mode, LogoutMode::None);
        assert_eq!(p.punishment.damage, 5.0);
        assert_eq!(p.triggers.tag_mode, TagMode::Both);
        assert!(p.restrictions.mending.disabled_in_combat);
        assert_eq!(parsed.command_restrictions.commands, vec!["tpa".to_string()]);
        assert_eq!(p.messages.combat_log, d.messages.combat_log);
        assert_eq!(p.messages.timer.bossbar_title, d.messages.timer.bossbar_title);
        assert_eq!(parsed.ignored_worlds, vec!["ffa".to_string()]);
        assert_eq!(parsed.anti_kill_abuse.limits.max_kills, 5);
        assert_eq!(parsed.anti_kill_abuse.punish_commands.len(), 2);
        assert_eq!(
            parsed.command_messages.status_title,
            plain.command_messages.status_title
        );
    }

    #[test]
    fn defaults_are_the_original_plugins_shipped_config() {
        let c = Config::default();
        let l = &c.combat_log;
        assert!(l.restrictions.elytra.disabled_in_combat);
        assert!(!l.restrictions.teleporting.disabled_in_combat);
        assert!(!l.restrictions.teleporting.allow_enderpearl);
        assert!(!l.restrictions.riptide.stop);
        assert_eq!(l.restrictions.riptide.cooldown, 10_000);
        assert_eq!(l.punishment.logout.mode, LogoutMode::None);
        assert!(l.triggers.enderpearl.set_combat_on_land);
        assert!(!l.triggers.enderpearl.only_if_already_in_combat);
        assert!(l.triggers.untag_on_kill);
        assert_eq!(l.timer.display, Display::Actionbar);
        assert!(l.tag_effects.is_empty());
        assert!(c.anti_kill_abuse.enabled);
    }

    #[test]
    fn tag_effects_and_nested_sections_parse() {
        let cfg = Config::parse(
            "[combat-log.timer]\ndisplay = \"bossbar\"\n[[combat-log.tag-effects]]\ntype = \"SLOWNESS\"\namplifier = 1\n[[combat-log.tag-effects]]\ntype = \"minecraft:weakness\"\nshow-particles = false\n",
        )
        .unwrap();
        assert_eq!(cfg.combat_log.timer.display, Display::Bossbar);
        assert_eq!(cfg.combat_log.tag_effects.len(), 2);
        assert_eq!(cfg.combat_log.tag_effects[0].duration_seconds, 15);
        assert_eq!(cfg.combat_log.tag_effects[0].amplifier, 1);
        assert!(!cfg.combat_log.tag_effects[1].show_particles);
        assert_eq!(Config::default().riptide_cooldown_ticks(), 200);
    }

    #[test]
    fn missing_keys_fall_back_and_bad_values_are_errors() {
        let cfg = Config::parse("[combat-log.timer]\nduration-seconds = 30\n").unwrap();
        assert_eq!(cfg.duration_ticks(), 600);
        assert_eq!(cfg.combat_log.punishment.damage, 5.0);
        assert!(Config::parse("[combat-log.punishment.logout]\nmode = \"explode\"\n").is_err());
    }

    #[test]
    fn blacklist_blocks_the_listed_command_in_any_spelling() {
        let cfg = Config::default();
        assert!(cfg.command_blocked("tpa Steve"));
        assert!(cfg.command_blocked("/TPA"));
        assert!(cfg.command_blocked("minecraft:tpa x"));
        assert!(cfg.command_blocked("essentials:tpa x"));
        assert!(!cfg.command_blocked("msg Steve hi"));
        assert!(!cfg.command_blocked(""));
    }

    #[test]
    fn whitelist_blocks_everything_else() {
        let cfg = Config::parse("[command-restrictions]\nmode = \"whitelist\"\ncommands = [\"msg\"]\n").unwrap();
        assert!(!cfg.command_blocked("msg Steve hi"));
        assert!(cfg.command_blocked("home"));
    }

    #[test]
    fn dimensions_and_operators() {
        let cfg = Config::parse(
            "ignored-worlds = [\"ffa\", \"minecraft:the_end\"]\n[exemptions]\nbypass-min-permission = 2\n",
        )
        .unwrap();
        assert!(cfg.dimension_ignored("minecraft:the_end"));
        assert!(cfg.dimension_ignored("custom:ffa"));
        assert!(!cfg.dimension_ignored("minecraft:overworld"));
        assert!(cfg.bypasses(2));
        assert!(!cfg.bypasses(1));
        let off = Config::parse("[exemptions]\nbypass-min-permission = 0\n").unwrap();
        assert!(!off.bypasses(4));
    }
}
