# tak-server

This is a rewrite of the java server at https://github.com/USTakAssociation/playtak-api in rust.

Missing features:

- [x] Chat rooms
- [ ] Registration
- [ ] Tests
- [ ] Rematches
- [ ] Mod and admin privileges
- [x] Player inactivity disconnect
- [ ] Password reset
- [ ] Broadcasting (is that in use?)
- [ ] IRCBridge (is that in use?)

To do extra features:

- [ ] JSON protocol alternative
- [ ] REST endpoints
- [ ] JWT auth

Open questions / On hold:

- [ ] Player ratings

- Rating computation is done in typescript service, values set in java code are simply overwritten. The old server computes a value (which seems to differ from the typescript service? Also contains database indirection (via ratingbase) for no good reason?) as a placeholder for games until the rating service is run. Wouldn't a clearly invalid value be better than an incorrect one?

- Games DB uses autoincrement INTEGER PRIMARY KEY, Players DB uses non-autoincrement INT PRIMARY_KEY. Why?