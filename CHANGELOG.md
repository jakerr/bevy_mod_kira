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
