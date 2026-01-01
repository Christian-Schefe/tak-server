# Architecture 

At the highest level, the code follows a hexagonal architecture (a.k.a ports and adapters).
Hexagonal architecture basically describes a three-layered architecture where dependency inversion is used for the lowest layer,
such that dependencies point towards the middle layer:


**API layer** --uses app interface--> **App layer (Core)** <--implements ports-- **Infra layer**



Advantages of such an architecture are as follows:

- Swapping adapters is (trivially) easy. For example, supplying a "log email adapter" that doesn't send emails, but logs them instead.
- The app layer has no architectural dependencies, which allows for easy testing by swapping out adapters with mock implementations.

## Adapters

Adapters make up the bottom infrastructure layer. Each adapter lives in its own crate, listed as follows:

- `tak-auth-ory` implements authentication with an external authentication system (specifically ory-kratos)
- `tak-email-lettre` implements sending emails with the `lettre`-crate using gmail smtp
- `tak-events-google-sheets` implements the event repository by reading from a remote google sheets document
- `tak-persistence-sea-orm` implement various domain repositories using the `sea-orm`-crate ontop of a `mariadb` database
- `tak-persistence-sea-orm-entites` defines the orm entites as a separate crate to later facilitate a migration setup.

## Core

The core is contained in the crate `tak-server-app` and defines a set of ports.

