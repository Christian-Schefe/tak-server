use crate::{
    PortsCollection,
    errors::{AppResult, InternalError},
};
use better_errors::OneOf;
use tak_domain::{UserId, game_record::PlayerSnapshot};

pub async fn get_snapshot<Ports: PortsCollection + Send + Sync + 'static>(
    ports: &Ports,
    ctx: &Ports::RpCtx,
    user_id: UserId,
) -> AppResult<Option<PlayerSnapshot>, (InternalError,)> {
    let Some(account) = ports
        .authentication_port()
        .get_account_info(user_id)
        .await
        .map_err(OneOf::broaden)?
    else {
        return Ok(None);
    };
    let current_rating = match ports
        .rating_repository()
        .get_player_rating(ctx, user_id)
        .await
        .map_err(OneOf::broaden)?
    {
        Some(rating) => Some(rating.rating),
        None => None,
    };

    Ok(Some(PlayerSnapshot::new(
        user_id,
        account.username,
        current_rating,
    )))
}
