# tak-server

This is a rewrite of the java server at https://github.com/USTakAssociation/playtak-api in rust.

To do:

- [x] Chat rooms
- [x] Registration
- [ ] Tests
- [ ] Rematches
- [ ] Mod and admin privileges
- [x] Player inactivity disconnect
- [x] Password reset (behaviour is slightly different)
- [ ] Broadcasting (is that in use?)
- [ ] IRCBridge (is that in use?)

To do extra features:

- [ ] JSON protocol alternative
- [ ] REST endpoints
- [x] JWT auth

Open questions / On hold:

- [ ] Player ratings

- Rating computation is done in typescript service, values set in java code are simply overwritten. The old server computes a value (which seems to differ from the typescript service? Also contains database indirection (via ratingbase) for no good reason?) as a placeholder for games until the rating service is run. Should the java algorithm be copied as is, or be adjusted?

- The text protocol system can't handle passwords with spaces correctly.

- Games DB uses autoincrement INTEGER PRIMARY KEY, Players DB uses non-autoincrement INT PRIMARY_KEY. Why?

- Move undo: apply time increment?
