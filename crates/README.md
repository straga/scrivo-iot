# crates - code shared by the firmwares

What belongs to no single board lives here. The burner takes its **logic** and the board's services from here; its own layers are in `devices/burner` (see the `README.md` there).

| Group | Crates | What it is |
|---|---|---|
| burner logic | `automaton`, `automaton_card`, `thermostat`, `demand`, `schedule`, `regulator`, `flame`, `draught`, `max6675`, `modbus`, `burner_map`, `dimmer`, `pellets` | what the readings mean, the burn automaton, the draught regulator, the pellet count; no hardware, tests run on the computer. `automaton_card` builds the automaton's card for the board and for the page's stand alike |
| board services | `firmware`, `web`, `vfs`, `flash`, `journal`, `ways_down`, `app_desc`, `reset_why`, `fell_where`, `energy`, `roam`, `rescue`, `board_name`, `shell`, `events`, `loudness`, `version`, `plan`, `round` | network, page, console, journal, the image's description and version, the way down to the recovery image, the sizes the heap is spent by |
| contract | `page`, `module`, `dictionary` | the shape of an entity and of the dictionary, the contract of a module (`docs/CONTRACT_Device_Api.md`); `dictionary` holds the entries every device shares and the stand's values |
| mechanics | `curve`, `between`, `between_leaps`, `analog`, `relay`, `switch`, `settings`, `tally`, `bl0939` | rings of readings, queues to an owner, the window of samples, the owner of the relays, settings, counts by day and month |
