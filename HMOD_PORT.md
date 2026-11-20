# hmod port: combatlog
Parity: before 80% -> after 83% (about 104/125 items; method: commands, permission nodes, config keys, messages and listed behaviours of CombatLog 1.20 counted against the original; items blocked by a missing engine API, and the logout kill behind the off-by-default feature, count as not done)

Round 1
- Fixed:
  - `src/lib.rs` imports: removed `PlayerDisconnectedEvent`, `QuitReason` (absent on main); added `PlayerKickedEvent`, permission types.
  - `src/lib.rs` `player_disconnected` hook replaced by `player_kicked`: a kick inserts the player into `kicked`. The set is still never read, as before.
  - `src/lib.rs` `may_admin`: `permission_value` / `permission_level` fallback replaced by `permission_bool(...).unwrap_or(false)`; takes `&mut dyn ServerHost`. Callers in `player_command` and `player_commands_sent` pass `&mut *host`. The `level` parameter is now `_level` (unused).
  - `src/lib.rs` `permission_gather_nodes` (new hook) + `admin_node()` helper; `register_commands` uses `with_node(&admin_node())`.
- Permission nodes: `combatlog.bypass` -> Level = `exemptions.bypass-min-permission` (4 when 0), declared in `src/lib.rs` `permission_gather_nodes`; `combatlog.admin` -> Level = `cfg.admin.min_permission` at gather time (`Config::default()` gives 1), declared in `src/lib.rs` `permission_gather_nodes`.
- Gaps: none. (Audit round: see README "Known gaps"; end-crystal explosions, firework boost cancel, portal teleports.)
- Unsure:
  - The node default level is read at gather time. A later `/combatlog reload` that changes `min_permission` does not change the node's default. Under the built-in handler the old fallback followed the reloaded value.
  - `player_kicked` fires before `player_logged_out`, which clears the `kicked` entry again. The old `player_disconnected` ran after logout. The set is never read, so no visible effect, but the entry no longer survives the logout.
  - Gather-time config: if `State` is not loaded when nodes are gathered, the default is `Config::default()`, i.e. level 1.

Round 3 (100 % round, written blind: nothing compiled or run)
- Added: logout kill at the quit, behind cargo feature `horizon_player_leaving` (off by default; needs the `player_leaving` hook of horizon-rust/horizon#18762 in the pinned SDK). `player_leaving` kills (`kill <name>`) or runs the logout commands while the player is still in the world, so their items drop where they logged out, and announces; `player_logged_out` then finds the player handled (`left`). Without the feature the kill still waits for the next join. Turn the feature on once the pin carries the hook.
- Added: end crystal blasts tag the hurt player like a block explosion (`triggers.explosions.set-combat-on-explosion`): crystals are remembered from `entity_join_level` and kept 40 ticks after they leave (a crystal leaves the level before its blast hurts anyone); damage of type `explosion` whose direct entity is one counts.
- Added: portals are teleports: `entity_travel_to_dimension` with a cause that contains `portal` is refused in combat when `teleporting.disabled-in-combat` is on (new `Action::Portal`, same message). Unverified: the engine's cause strings.
- Fixed: kill-abuse counts every kill by a player, also when the killer is exempt or in an ignored dimension (every player hit is remembered for 5 seconds, `last_hit`), as `AntiKillAbuse` counts `getKiller()`.
- Fixed: `/combatlog status` prints the world name (`world`, `world_nether`, `world_the_end`, or the last part of the id) and `ignored-worlds` also matches those names (`config::world_name`).
- Gaps (GAPS_UPSTREAM.md): `entity_toggle_glide` and `player_elytra_boost` cannot be cancelled (a refused glide or boost only gets the message and the penalty); mending cannot be vetoed; a hit that deals no health damage is not heard; the pearl trigger still waits for a teleport (no projectile-hit hook for pearls); `combat-zone`, WorldGuard safe zone, AFK title and PlaceholderAPI placeholders are not ported.
