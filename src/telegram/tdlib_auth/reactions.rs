use crate::domain::reaction_picker_state::{is_allowed_reaction, AvailableReaction};
use crate::usecases::message_reactions::{
    AddReactionQuery, AvailableReactionsQuery, ReactionError,
};

use super::TdLibAuthBackend;

impl TdLibAuthBackend {
    pub fn get_available_reactions(
        &self,
        query: &AvailableReactionsQuery,
    ) -> Result<Vec<AvailableReaction>, ReactionError> {
        let available = self
            .client
            .get_message_available_reactions(query.chat_id, query.message_id)
            .map_err(|e| {
                tracing::debug!(error = ?e, "get_message_available_reactions failed");
                ReactionError::Unavailable
            })?;

        let mut reactions = Vec::new();
        for r in available
            .top_reactions
            .iter()
            .chain(available.recent_reactions.iter())
            .chain(available.popular_reactions.iter())
        {
            let emoji = match &r.r#type {
                tdlib_rs::enums::ReactionType::Emoji(e) => &e.emoji,
                tdlib_rs::enums::ReactionType::CustomEmoji(_) => continue,
                tdlib_rs::enums::ReactionType::Paid => continue,
            };
            if !is_allowed_reaction(emoji) {
                continue;
            }
            if reactions
                .iter()
                .any(|existing: &AvailableReaction| existing.emoji == *emoji)
            {
                continue;
            }
            reactions.push(AvailableReaction {
                emoji: emoji.clone(),
                needs_premium: r.needs_premium,
            });
        }

        Ok(reactions)
    }

    pub fn add_reaction(&self, query: &AddReactionQuery) -> Result<(), ReactionError> {
        let reaction_type =
            tdlib_rs::enums::ReactionType::Emoji(tdlib_rs::types::ReactionTypeEmoji {
                emoji: query.emoji.clone(),
            });

        self.client
            .add_message_reaction(query.chat_id, query.message_id, reaction_type)
            .map_err(|e| {
                tracing::debug!(error = ?e, "add_message_reaction failed");
                ReactionError::Unavailable
            })
    }

    #[allow(dead_code)]
    pub fn remove_reaction(&self, query: &AddReactionQuery) -> Result<(), ReactionError> {
        let reaction_type =
            tdlib_rs::enums::ReactionType::Emoji(tdlib_rs::types::ReactionTypeEmoji {
                emoji: query.emoji.clone(),
            });

        self.client
            .remove_message_reaction(query.chat_id, query.message_id, reaction_type)
            .map_err(|e| {
                tracing::debug!(error = ?e, "remove_message_reaction failed");
                ReactionError::Unavailable
            })
    }
}
