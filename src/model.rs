use alloc::borrow::Cow;
use core::fmt;
use core::num::NonZero;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serenity::model::id;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(transparent)]
pub struct MessageId(id::MessageId);

impl MessageId {
    pub const fn new(message_id: u64) -> Option<Self> {
        if message_id == 0 {
            None
        } else {
            Some(Self(id::MessageId::new(message_id)))
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl JsonSchema for MessageId {
    fn schema_name() -> String {
        String::from(stringify!(MessageId))
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed(stringify!(MessageId))
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <NonZero<u64> as JsonSchema>::json_schema(generator)
    }
}

impl From<NonZero<u64>> for MessageId {
    fn from(message_id: NonZero<u64>) -> Self {
        Self(id::MessageId::new(message_id.get()))
    }
}

impl From<MessageId> for id::MessageId {
    fn from(message_id: MessageId) -> Self {
        message_id.0
    }
}

impl From<id::MessageId> for MessageId {
    fn from(message_id: id::MessageId) -> Self {
        Self(message_id)
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.get(), fmt)
    }
}

impl fmt::Debug for MessageId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self.get(), fmt)
    }
}
