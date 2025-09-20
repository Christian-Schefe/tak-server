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
- [x] Game abandon after timeout on disconnect
- [ ] Tests

To do extra features:

- [x] REST endpoints
- [x] JWT auth
- [ ] JSON protocol alternative

Open questions / On hold:

- [ ] Broadcasting (is that in use?)
- [ ] IRCBridge (is that in use?)

- Old Java code does a complicated calculation for temporary rating that seems excessive, as it anyway overwritten by the typescript service.
- The text protocol system can't handle passwords with spaces.
- Games DB uses autoincrement INTEGER PRIMARY KEY, Players DB uses non-autoincrement INT PRIMARY_KEY. Why?
- Ban doesn't have any effect?
- What's the point of sudo broadcast?

Changes:

- Ban prevents login
- sudo broadcast is a noop
- sudo reload is a noop as profanity filter is now a library.
