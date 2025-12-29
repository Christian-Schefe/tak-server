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

## Concepts

Player: A player is an abstract domain entity able to participate in games, identified by a unique player_id. The player does not know who owns it. As of now, a player has no associated attributes, but it does own information about domain activity (e.g. played games, stats, etc.).

Account: An account is an app concept describing the entity using the account. A human player owns a standard account, a bot owns a bot account, a temporary guest owns a temporary guest account. Depending on the account type, an account may own at most one player through which the owning entity is able to play games. The mapping from account to player is owned by the app layer.
As an entity representing the user, the account owns information about representation and authentification (e.g. username, profile picture, email, auth methods, roles & permissions, etc.), but not about domain activity owned by the player entity.


## Feature wishlist

- Async games
- Player profiles
- Player game history
- Replay move timings
- Native tournaments
