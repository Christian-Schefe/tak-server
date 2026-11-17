use terrors::OneOf;

pub type AppResult<T, E> = Result<T, OneOf<E>>;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InternalError;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InvalidCredentials;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct AccountNotFound;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InvalidEmailAddress;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct EmailTaken;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct GameNotFound;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct GameNotOngoing;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct MatchNotFound;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InsufficientPermissions;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InvalidChatMessage;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct NotAPlayerInGame;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InvalidGameAction;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct GameRequestNotFound;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct GameCantAbort;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct InvalidGameSettings;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct PlayerNotFound;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct CantBeginGameInMatch;
