# horizon-combatlog

> **Unofficial Rust port of CombatLog for the Horizon server (https://github.com/horizon-rust/horizon). Not affiliated with or endorsed by the CombatLog authors.**

Combat tagging for [Horizon](https://github.com/horizon-rust/horizon) servers, as a native trait mod
(`docs/native-mods.md`, "Trait mods"). Derived from **CombatLog 1.20** by Nikey (https://github.com/NikeyV1/CombatLog,
GPL-3.0; `de.nikey`, Bukkit/Paper plugin, `plugin.yml` `api-version: 26.3`). Not affiliated with its author.

## What it does

- A player who hurts another player tags **both** (configurable) for 15 seconds; every further hit renews it.
  The countdown shows on the action bar (`14/15`).
- **Logging out while tagged kills the player.** The server announces `<name> has combat logged!`. The kill is carried
  out as the player leaves, so their items drop where they logged out. Alternatively the mod can run console commands
  (`{player}` = name) or do nothing. A kick counts as a logout, as in the original.
- **Mending** repairs nothing for a tagged player (the experience is not spent).
- **Blocked commands** (default `tpa`, as a blacklist or a whitelist) are refused while tagged, with the message only,
  as in the original (no damage). A command is matched by its name or without its namespace (`essentials:tpa` is `tpa`).
- **Killing ends combat**: when a player dies, their own tag ends. Their killer's tag ends too, unless the killer still
  fights someone else (`untag-on-kill`). A death without a player killer leaves the other fighters tagged.
- Operators (permission level 1 and up) are never tagged; dimensions can be excluded. A server shutdown is not combat
  logging.

Added in 0.2.0, **written blind (not built, not run on a server yet)**; all off by default, as on the server the
original was configured for:

- **Teleport restriction** (`teleport-disabled`): ender pearls, chorus fruit and end gateways are refused in combat
  (the engine's cancellable hooks); `/tp` and `/teleport` are only reported after the move, so the mod moves the
  player back. `allow-enderpearl` keeps pearls working. Nether and End portals are not teleports here (the Bukkit
  plugin's teleport listener does not see portal travel either).
- **Elytra restriction** (`elytra-disabled`): starting a glide in combat is answered with the message and the damage
  penalty. The engine cannot stop the glide yet (see below), so the player keeps gliding.
- **Riptide** (`riptide-stop`, `riptide-cooldown-ms`): a riptide launch in combat is ended by putting the player back
  where it started, and the trident stays locked for the cooldown, also after the tag ran out.
- A refused elytra start, teleport or riptide takes the damage penalty (`punishment.damage`); a blocked command does not.
- **Boss bar timer** (`timer.display = "bossbar"`): a red bar per tagged player counting down, through the vanilla
  `/bossbar` command; removed when the tag ends, the player leaves or the server stops.
- **Effects on tag** (`[[tag-effects]]`): `effect give` when a tag starts (Bukkit names like `SLOWNESS` work), given
  again when the tag is renewed, and `effect clear` for each when the tag ends.
- **Ender pearl trigger** (`enderpearl-set-combat-on-land`, `enderpearl-only-if-already-in-combat`).
- **Anti-kill-abuse** (`[anti-kill-abuse]`): more than `max-kills` kills of the same victim within
  `remove-kill-after-seconds` runs the punish commands (`{killer}`, `{victim}`); the killer is whoever hit the victim
  last within a tag's duration.
- **`/combatlog reload | status <player> | untag <player>`** for operators of `[admin] min-permission`, with the
  original's answers (MiniMessage colour tags such as `<red>` are understood in every message).

## Config

`config/combatlog-common.toml`, written with defaults when missing and reloaded within a second of saving. The key paths are those of the original's `config.yml` and `messages.yml`, written as TOML tables: `combat-log.timer.duration-seconds`
is `[combat-log.timer]` with `duration-seconds`. The defaults are the original's shipped defaults (the jar's `config.yml`).
Port-only keys: `[admin] min-permission` and `[exemptions] bypass-min-permission` stand in for the permission nodes
`combatlog.admin` and `combatlog.bypass` (both default to operators). The command answers are flat keys in `[command-messages]`
(the original's `commands.combatlog.*` paths, same texts).

The shipped defaults differ from the values the original ran with on its server: there elytra, the ender pearl trigger,
the explosion trigger and anti-kill-abuse were off and logout was `kill`. A server that wants that behaviour copies those
values into its config. Note: the shipped default of anti-kill-abuse is enabled and runs `ban`, so check it before going live.

## Build

A trait mod. Build it with the Horizon launcher (`horizon-launcher native-build <this folder> --server-output DIR
--version TAG --trust-native-code`) or by hand as the launcher does: put `api/` into the exported Horizon workspace,
point its `horizon-client` dependency at the workspace crate, and link `mod_combatlog::hooks()` into the game
executable. The bundle is pinned to the Horizon commit in `horizon-mixin.toml`.

The logic lives in its own crate, `api/core` (`combatlog-core`: serde and toml only, no game types), so its unit tests
run without Horizon: `cd api/core && cargo test`. The `api` crate is the glue to the hooks.

## Licence and origin

GPL-3.0, see `LICENSE`; the original's copyright notice is in `NOTICE`.

This mod is a port of CombatLog 1.20 (https://github.com/NikeyV1/CombatLog, GPL-3.0, Copyright (c) 2026 Nikey) and is
therefore a derived work under the same licence. Until this revision the port was a clean-room rebuild from the plugin.yml
and config; the logic since then follows the original's source code (`src/main/java/de/nikey/combatLog`, upstream commit
18c0fd1a0dbdd574f7c3651539126941d06fcf31) and is no longer clean room.

## Choices that are ours (the original may differ)

- A hit is any damage one player's attack or projectile does to another (the engine's "incoming damage" event).
- Mending is simply disabled, without damage.
- Death on logout happens at the moment of logout (`player_logging_out`, hooks-all); on an engine without that hook it
  falls back to the next join.
- Operators are exempt (the original's `combatlog.bypass` defaults to operators).
- 0.2.0: the riptide cooldown applies only with `riptide-stop` on (the original's documentation does not say what the
  cooldown does on its own); a stopped riptide is ended by a teleport to where it started.
- The original checks anti-kill-abuse every 25 seconds; this port checks at each kill.
- 0.2.0: `/combatlog status` always answers "no" for the combat zone and WorldGuard (neither exists in this port);
  `/combatlog reload` reads `config/combatlog-common.toml` at once (the file watcher also does within a second).
- 0.2.0: `/combatlog status` always answers "no" for the combat zone and WorldGuard (neither exists in this port);
  `/combatlog reload` reads `config/combatlog-common.toml` at once (the file watcher also does within a second).

## Not ported

- The **combat zone** (`combat-zone.enabled`, `radius`): off on the server this was made for, and its documentation
  does not say what it does.
- **WorldGuard safe-zone barriers** and the region-entry refusal: there is no WorldGuard on Horizon to ask for regions;
  a port of it would have to offer a "region at this position" query first.
- **PlaceholderAPI** placeholders: no placeholder service on Horizon.
- The explosion trigger (`combat-log.triggers.explosions.set-combat-on-explosion`, `true` in the shipped config): the damage
  hook names the causing entity, not whether it was an end crystal or a block explosion. Not ported yet.
- `commands.combatlog.no-permission`: the engine refuses `/combatlog` to non-operators itself; the original's message is not sent.
- The AFK title (`afk-title`): it belongs to an AFK plugin's state, which Horizon has no hook for.

## Proof (Horizon `69fab597a`, throwaway dedicated server, protocol clients, unmodded engine vs. engine with the mod)

| Trigger | Unmodded | With the mod |
|---|---|---|
| Player A hits player B | no action bar | both get a red `15/15`, `14/15`, ... on the action bar |
| B runs `tpa` while tagged | `Unknown or incomplete command` | `You can't use this command in combat`, health 19 -> 14 |
| B runs `list` while tagged | works | works |
| Mending while tagged (sword damage 500) | repaired (496) | not repaired (500), the experience becomes points |
| Mending after the 15 s ran out | repaired | repaired |
| Tagged B disconnects, then joins again | alive, items kept | server log `B has combat logged!`; on the next join B dies (`death.attack.genericKill`) and the 5 diamonds drop |
| Untagged player disconnects, joins again | alive | alive (20 health) |
| B dies while A is tagged | n/a | A's countdown stops |
| Config saved while running | n/a | `config reloaded`; an invalid value keeps the old config |

Unit tests (`cargo test`): tagging modes, renewal, expiry, untag-on-kill (this found a real ordering bug), exempt players,
command matching, config defaults and parsing, colour codes.

## Not proven

Everything added in 0.2.0 (see "What it does") is unbuilt and unproven: the glue in `api/src/lib.rs` was written
against the hook catalog of the pinned revision without compiling it. Only the logic in `api/core` has run (its unit
tests). In particular unproven: that the vanilla `/bossbar` command reaches a dedicated server's clients, that a
teleport ends a riptide launch, and the exact field names of the generated event structs.

- Projectile hits (bow, trident, snowball): the engine does not let a player's projectile hurt another player yet, so only
  melee was exercised. The mod also listens to the engine's incoming-damage event, which would cover projectiles.
- `logout-mode = "commands"`, `tag-mode` attacker/victim, `start-below-health`, whitelist mode and operator exemption are
  covered by unit tests of the logic only, not on a server.
- Ignored dimensions and a kicked player (a kick counts as combat logging, as in the original) are not exercised on a server.
- This revision's changes (per-victim kill abuse, killer-only untag, blocked commands without damage, effects cleared and
  renewed, the health threshold for fights already on, `/combatlog status|untag` usage lines) were not compiled or run:
  no `cargo` was run for them.
- The mod was built by hand into the exported Horizon workspace the way the launcher does it, not with the launcher itself.

## What the engine does not offer yet (workarounds in the mod)

- (Solved with hooks-all) `player_logging_out` kills a tagged player while they are still in their level, so the drops
  land where they logged out; the next-join kill stays only as a fallback.
- (Solved with hooks-all) `player_hurt_by_player` names attacker and victim for melee, sweep and projectiles past the
  pvp rule, teams and invulnerability, so the `attack_entity` + health-drop guess is gone. Hits are still resolved at
  the next tick (the hook has no host).
- `entity_toggle_glide` only listens: a cancellable glide start (or a `stop_gliding(uuid)` helper) is needed to really
  refuse the elytra.
- `player_teleported` only listens, after the move: a cancellable command-teleport hook would replace the move back.
- `player_riptide` only listens, after the velocity is applied, and `entity_set_velocity` refuses players: a cancellable
  riptide hook (or a player velocity helper) would replace the teleport.
- No boss bar helper: the timer goes through `/bossbar`, whose bars are saved with the world (a crash leaves one
  behind until `/bossbar remove`). A per-player boss bar helper would avoid both.

## Contributing

Pull requests and improvements are always welcome! Open an issue or a PR -- direct pushes to main are disabled, every change goes through review.
