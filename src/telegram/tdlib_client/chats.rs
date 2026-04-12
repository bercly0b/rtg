use super::types::{TdLibError, TDLIB_ERROR_ALL_CHATS_LOADED};
use super::TdLibClient;

pub struct GetChatsResult {
    pub chat_ids: Vec<i64>,
    pub all_loaded: bool,
}

impl TdLibClient {
    /// Gets list of chat IDs from TDLib.
    ///
    /// Returns up to `limit` chat IDs from the main chat list, sorted by TDLib's order.
    /// First attempts `loadChats` to fetch from the server, then reads local
    /// cache via `getChats`. If `loadChats` fails (e.g. no network), we still
    /// try `getChats` to return whatever is available from TDLib's local
    /// SQLite database — this keeps the chat list usable in offline scenarios.
    ///
    /// `all_loaded` is `true` when TDLib signals there are no more chats to fetch.
    pub fn get_chats(&self, limit: i32) -> Result<GetChatsResult, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let mut all_loaded = false;

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
                    all_loaded = true;
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
                tdlib_rs::enums::Chats::Chats(c) => {
                    // `loadChats` 404 means "no more chats on the server", but
                    // TDLib's local cache may still hold more chats than we
                    // requested via `getChats(limit)`. Only report all_loaded
                    // when the cache is also exhausted (returned fewer than
                    // requested).
                    let actually_all_loaded = all_loaded && (c.chat_ids.len() as i32) < limit;
                    Ok(GetChatsResult {
                        chat_ids: c.chat_ids,
                        all_loaded: actually_all_loaded,
                    })
                }
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
