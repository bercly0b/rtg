use super::types::TdLibError;
use super::TdLibClient;

impl TdLibClient {
    /// Finds known chats by title and username via `searchChats`; an offline
    /// request.
    pub fn search_chats(&self, query: &str, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;
        let query = query.to_owned();

        self.block_on_request("searchChats", async {
            let tdlib_rs::enums::Chats::Chats(chats) =
                tdlib_rs::functions::search_chats(query, limit, client_id).await?;
            Ok(chats.chat_ids)
        })
    }

    /// Finds the user's chats by title and username via
    /// `searchChatsOnServer`.
    pub fn search_chats_on_server(&self, query: &str, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;
        let query = query.to_owned();

        self.block_on_request("searchChatsOnServer", async {
            let tdlib_rs::enums::Chats::Chats(chats) =
                tdlib_rs::functions::search_chats_on_server(query, limit, client_id).await?;
            Ok(chats.chat_ids)
        })
    }

    /// Finds public chats by username and title via `searchPublicChats`.
    /// TDLib leaves out the chats of the chat list and private chats with
    /// contacts, and chooses the number of results itself.
    pub fn search_public_chats(&self, query: &str) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;
        let query = query.to_owned();

        self.block_on_request("searchPublicChats", async {
            let tdlib_rs::enums::Chats::Chats(chats) =
                tdlib_rs::functions::search_public_chats(query, client_id).await?;
            Ok(chats.chat_ids)
        })
    }

    /// Finds contacts by name and username via `searchContacts`; returns
    /// user ids.
    pub fn search_contacts(&self, query: &str, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;
        let query = query.to_owned();

        self.block_on_request("searchContacts", async {
            let tdlib_rs::enums::Users::Users(users) =
                tdlib_rs::functions::search_contacts(query, limit, client_id).await?;
            Ok(users.user_ids)
        })
    }

    /// Returns the private chat with a user via `createPrivateChat` with
    /// `force`: no server request, so only the chat's id, type and title are
    /// reliable. Such a chat joins no chat list.
    pub fn create_private_chat_offline(
        &self,
        user_id: i64,
    ) -> Result<tdlib_rs::types::Chat, TdLibError> {
        let client_id = self.client_id;

        self.block_on_request("createPrivateChat", async {
            let tdlib_rs::enums::Chat::Chat(chat) =
                tdlib_rs::functions::create_private_chat(user_id, true, client_id).await?;
            Ok(chat)
        })
    }

    /// Finds messages in all chats except secret ones via `searchMessages`,
    /// newest first: the main list and the archive alike, without filters.
    /// `offset` is empty for the first page, or the `next_offset` of the
    /// page before.
    pub fn search_messages(
        &self,
        query: &str,
        offset: &str,
        limit: i32,
    ) -> Result<tdlib_rs::types::FoundMessages, TdLibError> {
        let client_id = self.client_id;
        let query = query.to_owned();
        let offset = offset.to_owned();

        self.block_on_request("searchMessages", async {
            let tdlib_rs::enums::FoundMessages::FoundMessages(found) =
                tdlib_rs::functions::search_messages(
                    None, query, offset, limit, None, None, 0, 0, client_id,
                )
                .await?;
            Ok(found)
        })
    }
}
