use crate::domain::reaction_picker_state::AvailableReaction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableReactionsQuery {
    pub chat_id: i64,
    pub message_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddReactionQuery {
    pub chat_id: i64,
    pub message_id: i64,
    pub emoji: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactionError {
    Unavailable,
}

pub trait ReactionSource: Send + Sync {
    fn get_available_reactions(
        &self,
        query: &AvailableReactionsQuery,
    ) -> Result<Vec<AvailableReaction>, ReactionError>;

    fn add_reaction(&self, query: &AddReactionQuery) -> Result<(), ReactionError>;

    fn remove_reaction(&self, query: &AddReactionQuery) -> Result<(), ReactionError>;
}

impl<T: ReactionSource> ReactionSource for std::sync::Arc<T> {
    fn get_available_reactions(
        &self,
        query: &AvailableReactionsQuery,
    ) -> Result<Vec<AvailableReaction>, ReactionError> {
        (**self).get_available_reactions(query)
    }

    fn add_reaction(&self, query: &AddReactionQuery) -> Result<(), ReactionError> {
        (**self).add_reaction(query)
    }

    fn remove_reaction(&self, query: &AddReactionQuery) -> Result<(), ReactionError> {
        (**self).remove_reaction(query)
    }
}
