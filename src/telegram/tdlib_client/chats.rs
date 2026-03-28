use super::types::{TdLibError, TDLIB_ERROR_ALL_CHATS_LOADED};
use super::TdLibClient;

impl TdLibClient {
    /// Gets list of chat IDs from TDLib.
    ///
    /// Returns up to `limit` chat IDs from the main chat list, sorted by TDLib's order.
    /// First attempts `loadChats` to fetch from the server, then reads local
    /// cache via `getChats`. If `loadChats` fails (e.g. no network), we still
    /// try `getChats` to return whatever is available from TDLib's local
    /// SQLite database — this keeps the chat list usable in offline scenarios.
    pub fn get_chats(&self, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            // Try to load fresh chats from the server. Failures are non-fatal:
            // TDLib's local cache may still have chats from previous sessions.
            if let Err(e) = tdlib_rs::functions::load_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            {
                if e.code == TDLIB_ERROR_ALL_CHATS_LOADED {
                    tracing::debug!("load_chats returned 404: all chats already loaded");
                } else {
                    tracing::warn!(
                        code = e.code,
                        message = %e.message,
                        "load_chats failed; falling back to locally cached chats"
                    );
                }
            }

            // Read whatever chat IDs are available (server-fresh or locally cached).
            let chats = tdlib_rs::functions::get_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

            match chats {
                tdlib_rs::enums::Chats::Chats(c) => Ok(c.chat_ids),
            }
        })
    }

    /// Gets full chat information by ID.
    pub fn get_chat(&self, chat_id: i64) -> Result<tdlib_rs::types::Chat, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let chat = tdlib_rs::functions::get_chat(chat_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

            match chat {
                tdlib_rs::enums::Chat::Chat(c) => Ok(c),
            }
        })
    }

    /// Gets user information by ID.
    pub fn get_user(&self, user_id: i64) -> Result<tdlib_rs::types::User, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let user = tdlib_rs::functions::get_user(user_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

            match user {
                tdlib_rs::enums::User::User(u) => Ok(u),
            }
        })
    }

    /// Gets full user information (bio, photos, etc.) by user ID.
    pub fn get_user_full_info(
        &self,
        user_id: i64,
    ) -> Result<tdlib_rs::types::UserFullInfo, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let info = tdlib_rs::functions::get_user_full_info(user_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

            match info {
                tdlib_rs::enums::UserFullInfo::UserFullInfo(i) => Ok(i),
            }
        })
    }

    /// Gets full information about a supergroup or channel.
    pub fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<tdlib_rs::types::SupergroupFullInfo, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let info = tdlib_rs::functions::get_supergroup_full_info(supergroup_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

            match info {
                tdlib_rs::enums::SupergroupFullInfo::SupergroupFullInfo(i) => Ok(i),
            }
        })
    }

    /// Gets full information about a basic group.
    pub fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<tdlib_rs::types::BasicGroupFullInfo, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let info = tdlib_rs::functions::get_basic_group_full_info(basic_group_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

            match info {
                tdlib_rs::enums::BasicGroupFullInfo::BasicGroupFullInfo(i) => Ok(i),
            }
        })
    }

    /// Informs TDLib that the chat is opened by the user.
    ///
    /// Many useful activities depend on the chat being opened or closed
    /// (e.g., in supergroups and channels all updates are received only
    /// for opened chats). Must be paired with [`close_chat`](Self::close_chat).
    pub fn open_chat(&self, chat_id: i64) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::open_chat(chat_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }

    /// Informs TDLib that messages are being viewed by the user.
    ///
    /// This marks messages as read and updates view counters.
    /// The chat should be opened via [`open_chat`](Self::open_chat) before
    /// calling this method for `force_read: false` to work correctly.
    ///
    /// Uses `MessageSource::ChatHistory` as the source since messages
    /// are viewed from chat history in the TUI.
    pub fn view_messages(&self, chat_id: i64, message_ids: Vec<i64>) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::view_messages(
                chat_id,
                message_ids,
                Some(tdlib_rs::enums::MessageSource::ChatHistory),
                false, // force_read: false — rely on proper openChat/closeChat lifecycle
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })
        })
    }

    /// Informs TDLib that the chat is closed by the user.
    ///
    /// Must be called for every chat previously opened via
    /// [`open_chat`](Self::open_chat).
    pub fn close_chat(&self, chat_id: i64) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::close_chat(chat_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }
}
