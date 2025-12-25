
    fn model_to_rating(model: player::Model) -> PlayerRating {
        PlayerRating {
            rating: model.rating,
            boost: model.boost,
            max_rating: model.max_rating,
            rated_games_played: model.rated_games as u32,
            is_unrated: model.is_unrated,
            participation_rating: model.participation_rating as f64,
            rating_age: model.rating_age,
            fatigue: serde_json::from_str::<HashMap<uuid::Uuid, f64>>(&model.fatigue)
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (PlayerId(k), v))
                .collect(),
        }
    }