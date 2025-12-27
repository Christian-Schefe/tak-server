# tak-server

This is a rewrite of the java server at https://github.com/USTakAssociation/playtak-api in rust.

### Removed APIs:

- `sudo set password`

  Reason: Passwords should follow a self-service flow. Admins being able to set a password is an anti-pattern.

### Removed Responses

- `Accept rematch`

  Reason: Rematches are automatically started by the server, no need for the client to accept a rematch seek.
