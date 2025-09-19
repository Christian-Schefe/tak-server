# tak-server

This is a rewrite of the java server at https://github.com/USTakAssociation/playtak-api in rust.

In progress:

- Tests

To do:

- [x] Chat rooms
- [x] Registration
- [x] Profanity filter
- [x] Rematches
- [x] Mod and admin privileges
- [x] Player inactivity disconnect
- [x] Password reset
- [ ] Game abandon after timeout on disconnect
- [ ] Tests

To do extra features:

- [ ] JSON protocol alternativef
- [ ] REST endpoints
- [x] JWT auth

Open questions / On hold:

- [ ] Player ratings
- [ ] Broadcasting (is that in use?)
- [ ] IRCBridge (is that in use?)

- Rating computation is done in typescript service, values set in java code are simply overwritten. The old server computes a value (which seems to differ from the typescript service? Also contains database indirection (via ratingbase) for no good reason?) as a placeholder for games until the rating service is run. Should the java algorithm be copied as is, or be adjusted?

- The text protocol system can't handle passwords with spaces correctly.

- Games DB uses autoincrement INTEGER PRIMARY KEY, Players DB uses non-autoincrement INT PRIMARY_KEY. Why?

- Ban doesn't have any effect?
- What's the point of sudo broadcast?
