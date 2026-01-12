# tak-server

This is a rewrite of the java server at https://github.com/USTakAssociation/playtak-api in rust.

### Removed APIs:

- `sudo set password`

  Reason: Passwords should follow a self-service flow. Admins being able to set a password is an anti-pattern.

- `sudo bot/unbot`

  Reason: Bots are now identified by account type, not by a flag. Account type is set during account creation.

### Modified APIs:

- http: `GET /ratings`

  Modified: removed query parameters `id` and `name`

  Reason: There already is an endpoint for getting the rating via name, and if needed, an endpoint for getting ratings via id is trivially added. If at most one record is returned, there is no point in sorting and pagination.

### Removed Responses

- `Accept rematch`

  Reason: Rematches are automatically started by the server, no need for the client to accept a rematch seek.

### Other Changes

Removed "Tournament" flag from seeks and games (api still supports it, but ignores it)
Reason: Tournament system will be reworked, and tournament games won't be created by seeks

## Feature wishlist

- Async games (easy)
- Player profiles (easy)
- Player game history (easy)
- Native tournaments (medium because of scope. Architecturally it should be easy)
- Replay move timings (server side implemented)

# Todo Notes

- test rematches
- setup sea-orm migrations
