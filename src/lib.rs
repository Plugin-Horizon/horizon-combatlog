//! CombatLog as an arismoko native trait mod (`mod_api`). The logic is the unchanged
//! `combatlog-core` crate (`core/`); this file maps its hooks and calls onto arismoko's API.
//! Gaps and stand-ins are listed in PORT_NOTES.md and marked `TODO(arismoko-api: ...)` below.
//!
//! Hitting a player tags both sides for a while; a tagged player's logout kills them, their
//! Mending repairs nothing and the commands on the list are refused with a damage penalty.
//! Config: `config/combatlog-common.toml`, re-read when it changes. `/combatlog reload|status|untag`.
use combatlog_core::abuse::KillLog;
use combatlog_core::admin::{self, Admin};
use combatlog_core::cmds;
use combatlog_core::combat::{PlayerId, Tagger, effective_mode};
use combatlog_core::config::{Config, DEFAULT_TOML, Display, LogoutMode};
use combatlog_core::restrict::{self, Action, Guard};
use combatlog_core::text;
use mod_api::{
    Command, CommandArgumentValue, Config as NativeConfig, ConfigKind, ConfigLoadingEvent,
    ConfigReloadingEvent, EnderPearlTeleportEvent, EntityToggleGlideEvent, Flow, GameMode,
    HostSide, Hooks, ItemConsumptionTeleportEvent, LevelRandomHost, LivingDamagePostEvent,
    PlayerCommandEvent, PlayerDiedEvent, PlayerDisconnectedEvent,
    PlayerLoggedInEvent, PlayerLoggedOutEvent, PlayerRef, PlayerRiptideEvent, QuitReason,
    PlayerTeleportEndGatewayEvent, PlayerTeleportedEvent, RegisterCommandsEvent,
    RegisterConfigsEvent, RunModCommandEvent, ServerHost, ServerStoppingEvent, ServerTickEvent,
    ServerTickPhase, Text,
};
use mod_api::text::TextStyle;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Mutex;

const MOD_ID: &str = "combatlog";
/// The file the game keeps for this mod in the config folder (`/combatlog reload` re-reads it).
const CONFIG_PATH: &str = "config/combatlog-common.toml";

#[derive(Default)]
struct CombatLog {
    state: Mutex<State>,
}

/// What the last tick saw of a player: some hooks cannot reach the player list.
#[derive(Clone)]
struct Seen {
    uuid: PlayerId,
    /// The entity id the hurt and attack hooks carry.
    entity: u32,
    name: String,
    permission: u8,
    dimension: String,
    position: [f64; 3],
    game_mode: i32,
    health: f32,
}

#[derive(Default)]
struct State {
    cfg: Config,
    tagger: Tagger,
    guard: Guard,
    kills: KillLog,
    /// (attacker, victim) entity ids of a living hit that took health and had an attacker
    /// (`living_damage_post`), resolved at the next tick against the player list: a fight
    /// counts only between two players, teams and invulnerability as the original.
    hits: Vec<(u32, u32)>,
    /// Players whose logout was already handled by a logging-out hook (still in their level).
    /// Empty on arismoko: there is no such hook yet, see `player_logged_out`.
    left: BTreeSet<PlayerId>,
    /// The players as the last tick saw them: player hooks may run while the player list is out
    /// of reach, so they look players up here.
    players: Vec<Seen>,
    /// Players tagged outside the tick (a landing ender pearl): their tag start is handled there.
    fresh: BTreeSet<PlayerId>,
    /// Players whose timer is a boss bar.
    bars: BTreeSet<PlayerId>,
    /// Lines to tell a player at the next tick (refusals from hooks without a reply list).
    tells: Vec<(PlayerId, String)>,
    /// Console commands to run at the next tick, outside every hook.
    commands: Vec<String>,
    /// The effects CombatLog gave a tagged player (their name, the effect kinds): cleared when the
    /// tag ends, given again when the tag is renewed.
    effects: BTreeMap<PlayerId, (String, Vec<String>)>,
    /// Players whose tag was renewed outside the tick (a landing ender pearl).
    renew: BTreeSet<PlayerId>,
    /// Players who left while tagged with `logout-mode = "kill"`: killed when they join again.
    owed: BTreeMap<PlayerId, String>,
    owed_file: Option<PathBuf>,
    owed_loaded: bool,
    kicked: BTreeSet<PlayerId>,
    stopping: bool,
    ticks: u64,
}

impl State {
    /// Reads the owed kills once the world is open.
    fn load_owed(&mut self, host: &dyn ServerHost) {
        if self.owed_loaded {
            return;
        }
        let Some(world) = host.world_dir() else {
            return;
        };
        self.owed_loaded = true;
        let file = world.join("combatlog").join("owed-kills.txt");
        if let Ok(text) = std::fs::read_to_string(&file) {
            for line in text.lines() {
                if let Some((id, name)) = line.split_once('\t')
                    && let Ok(id) = u128::from_str_radix(id, 16)
                {
                    self.owed.insert(id, name.to_owned());
                }
            }
        }
        self.owed_file = Some(file);
    }

    fn save_owed(&self) {
        let Some(file) = &self.owed_file else {
            return;
        };
        let text: String = self
            .owed
            .iter()
            .map(|(id, name)| format!("{id:032x}\t{name}\n"))
            .collect();
        let saved = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(file, text));
        if let Err(e) = saved {
            eprintln!("[combatlog] could not save {}: {e}", file.display());
        }
    }

    fn exempt(&self, player: &Seen) -> bool {
        self.cfg.bypasses(player.permission) || self.cfg.dimension_ignored(&player.dimension)
    }

    fn player(&self, id: PlayerId) -> Option<&Seen> {
        self.players.iter().find(|p| p.uuid == id)
    }

    fn named(&self, name: &str) -> Option<&Seen> {
        self.players
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }

    fn timer_line(&self, id: PlayerId) -> Text {
        let left = self.tagger.seconds_left(id).to_string();
        let max = self.cfg.combat_log.timer.duration_seconds.to_string();
        legacy(&text::fill(
            &self.cfg.combat_log.messages.timer.actionbar,
            &[("timeLeft", &left), ("maxTime", &max)],
        ))
    }

    /// A refused action of `name`: the message at the next tick, and the damage penalty.
    fn refuse(&mut self, id: PlayerId, name: &str, message: String) {
        self.tells.push((id, message));
        if let Some(c) = cmds::penalty(&self.cfg, name) {
            self.commands.push(c);
        }
    }

    /// A tag just started: effects, and the boss bar when that is the timer.
    fn tag_started(&mut self, id: PlayerId, name: &str) {
        let effects = cmds::tag_effects(&self.cfg, name);
        let kinds: Vec<String> = self
            .cfg
            .combat_log
            .tag_effects
            .iter()
            .filter_map(cmds::effect_kind)
            .collect();
        if !kinds.is_empty() {
            self.effects.insert(id, (name.to_owned(), kinds));
        }
        self.commands.extend(effects);
        if self.cfg.combat_log.timer.display == Display::Bossbar && self.bars.insert(id) {
            let left = self.tagger.seconds_left(id);
            let create = cmds::bar_create(&self.cfg, id, name, left);
            self.commands.extend(create);
        }
    }

    /// A tag ended (timer, death, untag): the effects it gave are cleared, its boss bar goes at
    /// the next tick, or its action bar is cleared through `clear`.
    fn tag_ended(&mut self, id: PlayerId, clear: &mut Vec<PlayerId>) {
        if let Some((name, kinds)) = self.effects.remove(&id) {
            for kind in kinds {
                self.commands.push(cmds::effect_clear(&name, &kind));
            }
        }
        if self.bars.remove(&id) {
            self.commands.push(cmds::bar_remove(id));
        } else {
            clear.push(id);
        }
    }
}

/// `Text::empty()` of the original: an empty literal.
fn blank() -> Text {
    Text::literal("")
}

/// The `&`-coded template as coloured segments (`TextStyle` takes `#RRGGBB`).
fn legacy(template: &str) -> Text {
    text::segments(template)
        .into_iter()
        .fold(blank(), |all, (part, colour)| {
            let mut style = TextStyle::default();
            style.color = Some(format!("#{colour:06X}"));
            all.append(Text::literal(part).with_style(style))
        })
}

/// The vanilla id of a game mode, as `/combatlog status` shows it.
fn game_mode_id(mode: GameMode) -> i32 {
    match mode {
        GameMode::Survival => 0,
        GameMode::Creative => 1,
        GameMode::Adventure => 2,
        GameMode::Spectator => 3,
        _ => -1,
    }
}

/// What the player list says about `player` now; None when a read fails (the player is gone).
fn seen(host: &dyn ServerHost, player: PlayerRef) -> Option<Seen> {
    let entity = player.entity();
    Some(Seen {
        uuid: player.uuid(),
        entity: entity.id(),
        name: player.name(host)?,
        permission: player.permission_level(host)?,
        dimension: player.dimension(host)?.to_string(),
        position: entity.position(host)?,
        game_mode: game_mode_id(player.game_mode(host)?),
        // NOTE: a failed health read counts as 0, so a start threshold would tag such a player.
        health: host
            .living(entity)
            .and_then(|living| living.health(host))
            .unwrap_or(0.0),
    })
}

/// The connected player with this UUID, through the player list.
fn player_ref(host: &dyn ServerHost, id: PlayerId) -> Option<PlayerRef> {
    host.players()
        .server_players()
        .into_iter()
        .find(|p| p.uuid() == id)
}

fn tell(host: &mut dyn ServerHost, id: PlayerId, line: Text) {
    if let Some(player) = player_ref(host, id) {
        let _ = player.tell(host, line);
    }
}

fn action_bar(host: &mut dyn ServerHost, id: PlayerId, line: Text) {
    if let Some(player) = player_ref(host, id) {
        let _ = player.action_bar(host, line);
    }
}

fn lock(state: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// The `/combatlog` usages as arismoko's command syntax: the player argument is a plain word
/// (the lookup only finds online players, so a miss gets the "not found" answer).
fn usage(line: &str) -> String {
    line.replace("<player:player_name>", "<player:word>")
}

impl CombatLog {
    fn load_config(&self, text: &str, what: &str) -> bool {
        match Config::parse(text) {
            Ok(cfg) => {
                lock(&self.state).cfg = cfg;
                eprintln!("[combatlog] config {what}");
                true
            }
            Err(e) => {
                eprintln!("[combatlog] config not {what}, keeping the old one: {e}");
                false
            }
        }
    }

    /// A teleport of a tagged player that the engine lets a mod cancel.
    fn teleport_hook(&self, id: PlayerId, name: &str, action: Action) -> Flow {
        let mut st = lock(&self.state);
        let tagged = st.tagger.is_tagged(id);
        let Some(message) = restrict::refused(&st.cfg, action, tagged).map(str::to_owned) else {
            return Flow::Continue;
        };
        st.refuse(id, name, message);
        Flow::Cancel
    }
}

#[mod_api::horizon_mod]
impl Hooks for CombatLog {
    fn register_configs(&self, _: HostSide, event: &mut RegisterConfigsEvent<'_>) {
        event
            .configs
            .push(NativeConfig::new(MOD_ID, ConfigKind::Common, DEFAULT_TOML));
    }

    fn config_loading(&self, _: HostSide, event: &mut ConfigLoadingEvent<'_>) {
        if event.config.mod_id == MOD_ID {
            self.load_config(&event.config.text, "loaded");
        }
    }

    fn config_reloading(&self, _: HostSide, event: &mut ConfigReloadingEvent<'_>) {
        if event.config.mod_id == MOD_ID {
            self.load_config(&event.config.text, "reloaded");
        }
    }

    fn server_stopping(&self, host: &mut dyn ServerHost, _event: &mut ServerStoppingEvent) {
        // Everyone "logs out" when the server stops; that is not combat logging. Boss bars are
        // saved with the world, so ours go now.
        let bars: Vec<PlayerId> = {
            let mut st = lock(&self.state);
            st.stopping = true;
            std::mem::take(&mut st.bars).into_iter().collect()
        };
        for id in bars {
            host.commands().run_command(&cmds::bar_remove(id));
        }
    }

    /// Stands in for `player_hurt_by_player` (no such arismoko hook). Fires for every living hit
    /// that took health, so the attacker and victim are matched to players at the next tick.
    /// TODO(arismoko-api: player_hurt_by_player): the original also heard hits that the health
    /// check or invulnerability left at zero; this hears only hits that took health.
    fn living_damage_post(
        &self,
        _level: &mut dyn LevelRandomHost,
        event: &mut LivingDamagePostEvent<'_>,
    ) {
        let attacker = event.source.causing;
        let victim = event.entity;
        if event.container.health_damage() > 0.0 && attacker > 0 && victim > 0 && attacker != victim
        {
            lock(&self.state).hits.push((attacker as u32, victim as u32));
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickStart { .. }) {
            return;
        }
        let mut bars: Vec<(PlayerId, Text)> = Vec::new();
        let mut clear: Vec<PlayerId> = Vec::new();
        let commands;
        let tells;
        {
            let mut st = lock(&self.state);
            st.load_owed(&*host);
            st.ticks += 1;
            let ended = st.tagger.advance();
            let hits = std::mem::take(&mut st.hits);
            let mut fresh: BTreeSet<PlayerId> = std::mem::take(&mut st.fresh);
            let mut started: BTreeSet<PlayerId> = fresh.clone();
            let players: Vec<Seen> = host
                .players()
                .server_players()
                .into_iter()
                .filter_map(|p| seen(&*host, p))
                .collect();
            let find = |entity: u32| players.iter().find(|p| p.entity == entity);
            let mut fights: Vec<(&Seen, &Seen)> = Vec::new();
            for (attacker, victim) in hits {
                if let (Some(attacker), Some(victim)) = (find(attacker), find(victim)) {
                    fights.push((attacker, victim));
                }
            }
            for (attacker, victim) in fights {
                // The health threshold only starts a tag: a fight already on counts every hit.
                let below = st.cfg.combat_log.triggers.start_below_health;
                let in_combat =
                    st.tagger.is_tagged(attacker.uuid) || st.tagger.is_tagged(victim.uuid);
                if !in_combat && below > 0.0 && victim.health > below {
                    continue;
                }
                let Some(mode) = effective_mode(
                    st.cfg.combat_log.triggers.tag_mode,
                    !st.exempt(attacker),
                    !st.exempt(victim),
                ) else {
                    continue;
                };
                let ticks = st.cfg.duration_ticks();
                for id in st.tagger.hit(attacker.uuid, victim.uuid, mode, ticks) {
                    let name = if id == attacker.uuid {
                        &attacker.name
                    } else {
                        &victim.name
                    };
                    eprintln!("[combatlog] {name} is in combat");
                    started.insert(id);
                }
                for id in [attacker.uuid, victim.uuid] {
                    if st.tagger.is_tagged(id) {
                        fresh.insert(id);
                        if !started.contains(&id) {
                            st.renew.insert(id);
                        }
                    }
                }
            }
            st.players = players.clone();
            // Renewed tags (here or by a pearl) give their effects again; a new start gives them once.
            let renewed: Vec<PlayerId> = std::mem::take(&mut st.renew)
                .into_iter()
                .filter(|id| !started.contains(id))
                .collect();
            for id in started {
                if let Some(name) = st.player(id).map(|p| p.name.clone()) {
                    st.tag_started(id, &name);
                }
            }
            for id in renewed {
                if let Some(name) = st.effects.get(&id).map(|(n, _)| n.clone()) {
                    let again = cmds::tag_effects(&st.cfg, &name);
                    st.commands.extend(again);
                }
            }
            let second = st.ticks % 20 == 0;
            for id in st.tagger.tagged() {
                if !(second || fresh.contains(&id)) {
                    continue;
                }
                if st.bars.contains(&id) {
                    let left = st.tagger.seconds_left(id);
                    let update = cmds::bar_update(&st.cfg, id, left);
                    st.commands.extend(update);
                } else {
                    bars.push((id, st.timer_line(id)));
                }
            }
            if !ended.is_empty() {
                eprintln!("[combatlog] {} player(s) left combat", ended.len());
            }
            for id in ended {
                // A tag that ran out this tick and was hit again is still on: it did not end.
                if !st.tagger.is_tagged(id) {
                    st.tag_ended(id, &mut clear);
                }
            }
            commands = std::mem::take(&mut st.commands);
            tells = std::mem::take(&mut st.tells);
        }
        bars.extend(clear.into_iter().map(|id| (id, blank())));
        for (id, line) in bars {
            action_bar(host, id, line);
        }
        for (id, line) in tells {
            tell(host, id, legacy(&line));
        }
        for command in commands {
            host.commands().run_command(&command);
        }
    }

    fn player_command(&self, _host: &mut dyn ServerHost, event: &mut PlayerCommandEvent<'_>) -> Flow {
        let mut st = lock(&self.state);
        if !st.tagger.is_tagged(event.player) || !st.cfg.command_blocked(event.command) {
            return Flow::Continue;
        }
        // The original refuses a blocked command with the message only: no damage here.
        // Replies go out as system chat; the player helpers reach nobody in this hook.
        event.replies.push(legacy(&st.cfg.combat_log.messages.blocked_command));
        Flow::Cancel
    }

    // TODO(arismoko-api: mending_repair): upstream replaced the `mending_repair` hook with the item part
    // `ItemPart::xp_repair_ratio` (1e5c9bb062), asked only for parts attached to the stack's item, so a trait mod
    // can no longer refuse Mending to a player in combat. `restrictions.mending.disabled_in_combat` is not applied.

    fn ender_pearl_teleport(
        &self,
        host: &mut dyn ServerHost,
        event: &mut EnderPearlTeleportEvent<'_>,
    ) -> Flow {
        // Only a player's pearl counts: the thrower is the acting player (a mob's pearl is not).
        let Some(player) = host
            .players()
            .acting_player()
            .filter(|p| p.entity() == event.owner)
        else {
            return Flow::Continue;
        };
        let id = player.uuid();
        let name = player.name(&*host).unwrap_or_default();
        if matches!(
            self.teleport_hook(id, &name, Action::EnderPearl),
            Flow::Cancel
        ) {
            return Flow::Cancel;
        }
        // The pearl trigger: a landing pearl starts (or renews) its thrower's tag.
        let me = seen(&*host, player);
        let mut st = lock(&self.state);
        let t = &st.cfg.combat_log.triggers;
        if !t.enderpearl.set_combat_on_land {
            return Flow::Continue;
        }
        let only_renew = t.enderpearl.only_if_already_in_combat;
        let exempt = me.as_ref().is_none_or(|p| st.exempt(p));
        let ticks = st.cfg.duration_ticks();
        if !exempt {
            if st.tagger.tag_alone(id, ticks, only_renew) {
                st.fresh.insert(id);
            } else {
                st.renew.insert(id);
            }
        }
        Flow::Continue
    }

    fn item_consumption_teleport(
        &self,
        host: &mut dyn ServerHost,
        event: &mut ItemConsumptionTeleportEvent<'_>,
    ) -> Flow {
        // Chorus fruit (and the other `teleport_randomly` items) of a player; a fox is not one.
        let consumer = event.consumer.entity();
        let Some(player) = host
            .players()
            .acting_player()
            .filter(|p| p.entity() == consumer)
        else {
            return Flow::Continue;
        };
        let name = player.name(&*host).unwrap_or_default();
        self.teleport_hook(player.uuid(), &name, Action::ChorusFruit)
    }

    fn player_teleport_end_gateway(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut PlayerTeleportEndGatewayEvent<'_>,
    ) -> Flow {
        self.teleport_hook(event.player, event.name, Action::EndGateway)
    }

    fn player_teleported(&self, _host: &mut dyn ServerHost, event: &mut PlayerTeleportedEvent<'_>) {
        // Heard after the move: a refused /tp is undone by moving the player back, into the
        // dimension the last tick saw them in.
        let mut st = lock(&self.state);
        let tagged = st.tagger.is_tagged(event.player);
        let cfg = st.cfg.clone();
        if !st.guard.teleported(&cfg, event.player, tagged, event.to) {
            return;
        }
        let back_in = st
            .player(event.player)
            .map_or(event.dimension.to_owned(), |p| p.dimension.clone());
        st.guard.undoing(event.player, event.from);
        st.commands
            .push(cmds::move_back(event.name, &back_in, event.from));
        st.refuse(
            event.player,
            event.name,
            cfg.combat_log.messages.teleporting_denied.clone(),
        );
    }

    fn entity_toggle_glide(&self, _host: &mut dyn ServerHost, event: &mut EntityToggleGlideEvent<'_>) {
        // TODO(arismoko-api: entity_toggle_glide_cancel): the engine only tells; the glide cannot
        // be stopped, so a refused elytra only gets the message (and the penalty), as before.
        if !event.gliding {
            return;
        }
        let mut st = lock(&self.state);
        let tagged = st.tagger.is_tagged(event.player);
        if let Some(message) = restrict::refused(&st.cfg, Action::Elytra, tagged).map(str::to_owned)
        {
            st.refuse(event.player, event.name, message);
        }
    }

    fn player_riptide(&self, host: &mut dyn ServerHost, event: &mut PlayerRiptideEvent<'_>) {
        // The position the launch started from: the acting player's, as the original read it.
        let at = host
            .players()
            .acting_player()
            .filter(|p| p.uuid() == event.player)
            .and_then(|p| seen(&*host, p));
        let mut st = lock(&self.state);
        let tagged = st.tagger.is_tagged(event.player);
        let now = st.ticks;
        let cfg = st.cfg.clone();
        if !st.guard.riptide(&cfg, event.player, tagged, now) {
            return;
        }
        // No helper takes a player's velocity; a teleport to where the launch started is meant
        // to end it (unproven, see the README).
        if let Some(p) = at.or_else(|| st.player(event.player).cloned()) {
            st.commands
                .push(cmds::move_back(event.name, &p.dimension, p.position));
        }
        st.refuse(
            event.player,
            event.name,
            cfg.combat_log.messages.riptide_denied.clone(),
        );
    }

    fn player_died(&self, host: &mut dyn ServerHost, event: &mut PlayerDiedEvent<'_>) {
        let mut clear = Vec::new();
        {
            let mut st = lock(&self.state);
            // Read before the death forgets who hit whom.
            let killer = st.tagger.last_attacker(event.player);
            let on_kill = st.cfg.combat_log.triggers.untag_on_kill;
            for id in st.tagger.died(event.player, killer, on_kill) {
                st.tag_ended(id, &mut clear);
            }
            if let Some(killer) = killer {
                let cfg = st.cfg.anti_kill_abuse.clone();
                let now = st.ticks;
                let name = st.player(killer).map(|p| p.name.clone());
                let victim = st
                    .player(event.player)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                if st.kills.record(&cfg, killer, event.player, now)
                    && let Some(name) = name
                {
                    eprintln!("[combatlog] kill abuse by {name}");
                    st.commands.extend(cfg.punish_commands.iter().map(|c| {
                        text::fill(c, &[("killer", &name), ("victim", &victim)])
                    }));
                }
            }
        }
        for id in clear {
            action_bar(host, id, blank());
        }
    }

    /// The quit reason, heard right after `player_logged_out` for the same player (so it cannot
    /// change that hook's punishment). A kick is kept in `kicked`, as the kick hook kept it; the set
    /// is still never read, as before.
    fn player_disconnected(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut PlayerDisconnectedEvent<'_>,
    ) {
        let mut st = lock(&self.state);
        st.kicked.remove(&event.player);
        if event.reason == QuitReason::Kicked {
            st.kicked.insert(event.player);
        }
    }

    /// TODO(arismoko-api: player_logging_out): arismoko has no hook before the player leaves, so a
    /// tagged logout is punished here, after they left. The kill then waits for their next join
    /// (the original's fallback, `logout-mode = "kill"`) and the commands and announcement run now.
    fn player_logged_out(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        let announce = {
            let mut st = lock(&self.state);
            st.load_owed(&*host);
            st.guard.forget(event.player);
            if st.bars.remove(&event.player) {
                st.commands.push(cmds::bar_remove(event.player));
            }
            st.kicked.remove(&event.player);
            let handled = st.left.remove(&event.player);
            let was_tagged = st.tagger.untag(event.player);
            st.effects.remove(&event.player);
            if handled || st.stopping || !was_tagged {
                return;
            }
            match st.cfg.combat_log.punishment.logout.mode {
                LogoutMode::None => {}
                LogoutMode::Kill => {
                    st.owed.insert(event.player, event.name.to_owned());
                    st.save_owed();
                }
                LogoutMode::Commands => {
                    let commands: Vec<String> = st
                        .cfg
                        .combat_log
                        .punishment
                        .logout
                        .commands
                        .iter()
                        .map(|c| text::fill(c, &[("player", event.name)]))
                        .collect();
                    st.commands.extend(commands);
                }
            }
            legacy(&text::fill(
                &st.cfg.combat_log.messages.combat_log,
                &[("player", event.name)],
            ))
        };
        let _ = host.messages().broadcast(announce);
    }

    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        let mut st = lock(&self.state);
        st.load_owed(&*host);
        if st.owed.remove(&event.player).is_some() {
            st.save_owed();
            st.commands.push(format!("kill {}", event.name));
        }
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        let level = u8::try_from(lock(&self.state).cfg.admin.min_permission).unwrap_or(4);
        for line in admin::USAGES {
            event.commands.push(Command::op(&usage(line), level));
        }
    }

    /// The `/combatlog` command (`perform_command` of the original).
    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        if event.command.literals.first().map(String::as_str) != Some(admin::LABEL) {
            return Flow::Continue;
        }
        // Rebuilt as the typed line the core parses: the literal path, then the player name.
        let mut line = event.command.literals.join(" ");
        if let Some(CommandArgumentValue::String(name)) = event.command.argument("player") {
            line.push(' ');
            line.push_str(name);
        }
        let Some(command) = admin::parse(&line) else {
            return Flow::Continue;
        };
        let mut clear = Vec::new();
        let lines: Vec<String> = match command {
            Admin::Usage => admin::usage(&lock(&self.state).cfg),
            Admin::SubUsage(which) => vec![admin::sub_usage(&lock(&self.state).cfg, which)],
            Admin::Reload => {
                let text = std::fs::read_to_string(CONFIG_PATH).ok();
                let ok = text.is_some_and(|t| self.load_config(&t, "reloaded"));
                let st = lock(&self.state);
                let m = &st.cfg.command_messages;
                vec![if ok {
                    m.reload_success.clone()
                } else {
                    m.reload_failed.clone()
                }]
            }
            Admin::Status(name) => {
                let st = lock(&self.state);
                match st.named(&name) {
                    None => vec![admin::not_found(&st.cfg, &name)],
                    Some(p) => {
                        let left = st
                            .tagger
                            .is_tagged(p.uuid)
                            .then(|| st.tagger.seconds_left(p.uuid));
                        admin::status(
                            &st.cfg,
                            &admin::Status {
                                player: &p.name,
                                dimension: &p.dimension,
                                game_mode: p.game_mode,
                                seconds_left: left,
                                ignored_dimension: st.cfg.dimension_ignored(&p.dimension),
                            },
                        )
                    }
                }
            }
            Admin::Untag(name) => {
                let mut st = lock(&self.state);
                match st.named(&name).map(|p| (p.uuid, p.name.clone())) {
                    None => vec![admin::not_found(&st.cfg, &name)],
                    Some((id, real)) => {
                        let was = st.tagger.untag(id);
                        if was {
                            st.tag_ended(id, &mut clear);
                        }
                        vec![admin::untagged(&st.cfg, &real, was)]
                    }
                }
            }
        };
        for line in lines {
            let _ = host.commands().feedback(legacy(&line));
        }
        for id in clear {
            action_bar(host, id, blank());
        }
        Flow::Cancel
    }
}
