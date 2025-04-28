# 0.3.0

- **Library Updates**:
  - Upgraded `bevy` to version 0.16.0.
  - Upgraded `kira` to version 0.10.6.
  - Updated all other dependencies to their latest versions.

- **API Enhancements**:
  - Added support for playing sounds on specific tracks using the
  `KiraTrackHandle` component. This is to better allign with the Kira API. Track
  handles for sub-tracks should now be added wrapped in a `KiraTrackHandle`
  component to any entity that is convienent to associate with the track. When
  writing a KiraPlaySoundEvent, there is an optional `track_entity` field that
  can be used to target a specific track. If not provided, the sound will play
  on the main track (see the drum_machine example).

  - Enhanced the `KiraPlayable` trait to support playing sounds on main or custom tracks.

- **Examples**:
  - Updated examples to reflect the new API changes:

- **Code Refactoring**:
  - Simplified and modernized the codebase, including better error handling and updated function signatures.

# 0.2.0

- Upgrade to bevy 0.12
- Upgrade to kira 0.8
- Upgrade to egui 0.23

- In order to upgrade to Bevy 0.12 StaticSoundData from the Kira library needed to be wrapped in
  a new type `KiraStaticSoundData` so it could implement `TypePath` (required to be used as an
  asset) so places that use this type will need to access the inner anonymous field.

  i.e.:
  ```
  # before 0.2.0
  if let Some(sound_asset) = assets.get(&sound) { ... }
  # after 0.2.0
  if let Some(sound_asset) = assets.get(&sound.0) { ... }
  ```

# 0.1.2

- Renamed `KiraSoundHandle` to `KiraStaticSoundHandle`

- Removed the event based KiraAddClockEvent, KiraAddTrackEvent, KiraClocks and KiraTracks APIs,
  clocks and tracks should now be added in a startup system that interfaces with `KiraContext`
  directly. See the `drum_machine` example for reference.
  This allows using the full capability of the Kira tracks API including routing sub-tracks
  together.

- Added ability to play dynamic sounds that implement kira's Sound / SoundData APIs. See
  `play_dynamic_sound` example.

- Re-organized code into modules using the more recent recommendations for module naming (i.e.
  `foo.rs` along side a `foo` directory for any sub-modules, rather than `for/mod.rs`).

# 0.1.1

- Added feature flags for the supported static file loaders

# 0.1.0

- First release
