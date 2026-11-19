# hmod port: combatlog
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
