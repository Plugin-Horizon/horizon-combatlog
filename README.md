# horizon-combatlog

> **Unofficial Rust port of CombatLog for the Horizon server (https://github.com/arismoko/horizon). Not affiliated with or endorsed by the CombatLog authors.**

Combat tagging for [Horizon](https://github.com/arismoko/horizon) servers, as a native trait mod
(`docs/native-mods.md`, "Trait mods"). Derived from **CombatLog 1.20** by Nikey (https://github.com/NikeyV1/CombatLog,
GPL-3.0; `de.nikey`, Bukkit/Paper plugin, `plugin.yml` `api-version: 26.3`). Not affiliated with its author.

> **Built for and working only with [arismoko/horizon](https://github.com/arismoko/horizon)** (the Horizon fork that carries the mod API). [horizon-rust/horizon](https://github.com/horizon-rust/horizon) is experimental for mods and not supported: the earlier version for its native hooks / horizon-mixins is frozen on the `horizon-plain` branch (work stopped on 2026-11-12).

## Which Horizon

This is a native trait mod: `horizon-mod.toml`, a `Cargo.toml` that depends on `horizon-mod-api`, `src/lib.rs` with the hooks, and the plugin logic in the `core/` crate (plain Rust, no engine needed for its tests).

- Pin: `[native] horizon = "54090ee36c4e2c7b43b66db864220c577b94b7e0"` in `horizon-mod.toml` (game `26.3`). That is a development build of arismoko/horizon `main` plus mod API additions that are not all merged upstream yet, so a stock `main` may lack some host methods this mod calls until they land.
- It compiled and loaded on that build and was smoke-tested there; it has not seen a long run with real players. Treat it as experimental.
- Feature descriptions below were written for the earlier horizon-rust version; where they name engine calls (`native_*`) or old file paths, read them as descriptions only. The Known gaps list is current.

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

## Build

Build it as a native trait mod with arismoko's launcher (see the "native mods" how-to in the arismoko/horizon docs):

```sh
horizon-launcher native-build <path to this folder> --server-output <dir> --version <tag> --trust-native-code
```

`--trust-native-code` is your consent to compile and run this code inside the server. The logic crate tests run without the engine: `(cd core && cargo test)`.

## Known gaps

What does not work yet on arismoko/horizon (each is marked `TODO(arismoko-api: ...)` in `src/lib.rs`):

- Mending cannot be refused in combat (`restrictions.mending.disabled_in_combat` is not applied): arismoko moved Mending to an item part a trait mod cannot veto.
- A refused elytra glide cannot be stopped; the player only gets the message and the penalty.
- There is no hook before a player leaves, so a tagged logout is punished after they left: the kill waits for their next join, the commands and the announcement run at once.
- Hits that the health check or invulnerability left at zero are not heard.

## Licence and origin

GPL-3.0, see `LICENSE`; the original's copyright notice is in `NOTICE`.

This mod is a port of CombatLog 1.20 (https://github.com/NikeyV1/CombatLog, GPL-3.0, Copyright (c) 2026 Nikey) and is
therefore a derived work under the same licence. Until this revision the port was a clean-room rebuild from the plugin.yml
and config; the logic since then follows the original's source code (`src/main/java/de/nikey/combatLog`, upstream commit
18c0fd1a0dbdd574f7c3651539126941d06fcf31) and is no longer clean room.
